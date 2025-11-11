/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

#define G_LOG_DOMAIN "pt-application"

#define CONCURRENCY_LIMIT 3
#define THUMBNAILING_DONE_BATCH 10

#include "pt-config.h"

#include "phosh-thumbnailer-service.h"
#include "application.h"

#include <glib-unix.h>
#include <gtk/gtk.h>
#define GNOME_DESKTOP_USE_UNSTABLE_API
#include <libgnome-desktop/gnome-desktop-thumbnail.h>

/**
 * PtApplication:
 *
 * Provides a D-Bus service to thumbnail directory and files.
 */

struct _PtApplication {
  GApplication       parent;

  PtImplThumbnailer *impl;

  GCancellable      *cancel;
  GnomeDesktopThumbnailFactory *factory;
  gboolean     hold;
  GQueue      *queue;

  uint         len;
  GVariantDict thumbnails;
};

G_DEFINE_TYPE (PtApplication, pt_application, G_TYPE_APPLICATION);


static void inline log_error (GError *error, const char *format, ...) G_GNUC_PRINTF (2, 3);

static void inline
log_error (GError *error, const char *format, ...)
{
  va_list args;
  GLogLevelFlags level = G_LOG_LEVEL_WARNING;

  va_start (args, format);

  if (g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
    level = G_LOG_LEVEL_DEBUG;

  g_logv (G_LOG_DOMAIN, level, format, args);

  va_end (args);
}


/* Inspired by Nautilus:
 * https://gitlab.gnome.org/GNOME/nautilus/-/blob/220a3f644d397937da3895e9a80dc1ca7c70f3c9/src/nautilus-thumbnails.c#L136 */
static GnomeDesktopThumbnailSize
get_thumbnail_size (void)
{
  GdkDisplay *display;
  GListModel *monitors;
  int max_scale = 0;

  gtk_init ();

  display = gdk_display_open (NULL);
  if (!display) {
    g_warning ("Failed to open display");
    return GNOME_DESKTOP_THUMBNAIL_SIZE_NORMAL;
  }

  monitors = gdk_display_get_monitors (display);
  for (uint i = 0; i < g_list_model_get_n_items (monitors); i++) {
    g_autoptr (GdkMonitor) monitor = g_list_model_get_item (monitors, i);
    max_scale = MAX (max_scale, gdk_monitor_get_scale_factor (monitor));
  }

  gdk_display_close (display);

  if (max_scale <= 1)
    return GNOME_DESKTOP_THUMBNAIL_SIZE_LARGE;
  else if (max_scale <= 2)
    return GNOME_DESKTOP_THUMBNAIL_SIZE_XLARGE;
  else
    return GNOME_DESKTOP_THUMBNAIL_SIZE_XXLARGE;
}

/**
 * The thumbnailing operation works in a queue-based logic.
 *
 * - When `ThumbnailDirectory` is called, we enumerate all files in the directory.
 * - Add the found `GFile`s to the queue.
 * - Kick off queue processing with `CONCURRENCY_LIMIT` elements at a time.
 *
 * - When `ThumbnailFiles` is called, we loop through all given files.
 * - Create `GFile` out of each URI and add to the queue.
 * - Kick off queue processing with `CONCURRENCY_LIMIT` elements at a time.
 *
 * - On the `process_queue` function, it pops given number of elements off the queue.
 * - Start the thumbnailing process on them.
 * - Once a file is thumbnailed, then it calls `process_queue` but with size 1.
 * - This way, at any given time, no more than `CONCURRENCY_LIMIT` files are thumbnailed at a time.
 *
 * - By design of the service, only one thumbnailing request is handled at a time.
 * - It means when `ThumbnailDirectory` or `ThumbnailFiles` is called, they cancel the on-going
 *   thumbnailing operation and clear the queue.
 * - `StopThumbnailing` can be used to explicitly cancel the on-going thumbnailing operation and
 *   clear the queue.
 */
static void process_queue (uint size);

typedef struct {
  char  *uri;
  char  *mime_type;
  time_t mtime;
} FileInfo;


static void
file_info_free (FileInfo *info)
{
  g_free (info->uri);
  g_free (info->mime_type);
  g_free (info);
}


static void
emit_thumbnailing_done (PtApplication *self)
{
  GVariant *thumbnails;
  uint len;

  if (self->len < THUMBNAILING_DONE_BATCH && g_queue_get_length (self->queue) != 0)
    return;

  thumbnails = g_variant_dict_end (&self->thumbnails);
  len = self->len;
  self->len = 0;

  g_debug ("Emitting ThumbnailingDone for %d files", len);
  pt_impl_thumbnailer_emit_thumbnailing_done (self->impl,
                                              thumbnails,
                                              g_variant_new ("a{sv}", NULL));
}


static void
on_save_thumbnail_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  FileInfo *info = data;
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  g_autoptr (GError) error = NULL;
  gboolean success = FALSE;
  g_autofree char *thumbnail_path = NULL;

  success = gnome_desktop_thumbnail_factory_save_thumbnail_finish (self->factory, result, &error);
  if (!success) {
    log_error (error, "Failed to save thumbnail for %s: %s", info->uri, error->message);
  } else {
    g_debug ("Saved thumbnail for %s", info->uri);

    if (self->len == 0)
      g_variant_dict_init (&self->thumbnails, NULL);

    thumbnail_path = gnome_desktop_thumbnail_factory_lookup (self->factory, info->uri, info->mtime);
    g_variant_dict_insert (&self->thumbnails, info->uri, "s", thumbnail_path);
    self->len += 1;
    emit_thumbnailing_done (self);
  }

  file_info_free (info);
  process_queue (1);
}


static void
on_create_failed_thumbnail_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  FileInfo *info = data;
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  g_autoptr (GError) error = NULL;
  gboolean success = FALSE;

  success = gnome_desktop_thumbnail_factory_create_failed_thumbnail_finish (self->factory, result,
                                                                            &error);
  if (!success)
    log_error (error, "Failed to create failed thumbnail for %s: %s", info->uri, error->message);
  else
    g_debug ("Created failed thumbnail for %s", info->uri);

  file_info_free (info);
  process_queue (1);
}


static void
on_generate_thumbnail_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  FileInfo *info = data;
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  g_autoptr (GError) error = NULL;
  GdkPixbuf *thumbnail = NULL;

  thumbnail = gnome_desktop_thumbnail_factory_generate_thumbnail_finish (self->factory, result,
                                                                         &error);
  if (!thumbnail) {
    log_error (error, "Failed to thumbnail %s: %s", info->uri, error->message);
    gnome_desktop_thumbnail_factory_create_failed_thumbnail_async (self->factory, info->uri,
                                                                   info->mtime,
                                                                   self->cancel,
                                                                   on_create_failed_thumbnail_ready,
                                                                   info);
    return;
  }

  gnome_desktop_thumbnail_factory_save_thumbnail_async (self->factory, thumbnail, info->uri,
                                                        info->mtime,
                                                        self->cancel, on_save_thumbnail_ready,
                                                        info);
}


static void
start_thumbnailing_file (FileInfo *info)
{
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  g_autofree char *thumbnail_uri = NULL;

  g_debug ("To thumbnail file %s (%s; %ld)", info->uri, info->mime_type, info->mtime);

  thumbnail_uri = gnome_desktop_thumbnail_factory_lookup (self->factory, info->uri, info->mtime);
  if (thumbnail_uri) {
    g_debug ("Skipping %s as it has a valid thumbnail already", info->uri);
    file_info_free (info);
    process_queue (1);
    return;
  }

  if (gnome_desktop_thumbnail_factory_has_valid_failed_thumbnail (self->factory, info->uri,
                                                                  info->mtime)) {
    g_debug ("Skipping %s as it has a failed thumbnail already", info->uri);
    file_info_free (info);
    process_queue (1);
    return;
  }

  if (!gnome_desktop_thumbnail_factory_can_thumbnail (self->factory, info->uri, info->mime_type,
                                                      info->mtime)) {
    g_debug ("Skipping %s as it can not be thumbnailed", info->uri);
    file_info_free (info);
    process_queue (1);
    return;
  }

  gnome_desktop_thumbnail_factory_generate_thumbnail_async (self->factory, info->uri,
                                                            info->mime_type,
                                                            self->cancel,
                                                            on_generate_thumbnail_ready,
                                                            info);
}


static void
on_query_info_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  g_autoptr (GFile) file = G_FILE (source);
  g_autoptr (GError) error = NULL;
  g_autoptr (GFileInfo) file_info = NULL;
  FileInfo *info;

  file_info = g_file_query_info_finish (file, result, &error);
  if (!file_info) {
    log_error (error, "Failed to query info: %s", error->message);
    process_queue (1);
    return;
  }

  info = g_new0 (FileInfo, 1);
  info->uri = g_file_get_uri (file);
  info->mime_type = g_strdup (g_file_info_get_content_type (file_info));
  info->mtime = g_file_info_get_attribute_uint64 (file_info, G_FILE_ATTRIBUTE_TIME_MODIFIED);

  start_thumbnailing_file (info);
}


static void
process_queue (uint size)
{
  PtApplication *self = PT_APPLICATION (g_application_get_default ());

  for (uint i = 0; i < size; i++) {
    GFile *file = g_queue_pop_head (self->queue);
    if (!file)
      return;
    g_file_query_info_async (file,
                             G_FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE ","
                             G_FILE_ATTRIBUTE_TIME_MODIFIED, G_FILE_QUERY_INFO_NONE,
                             G_PRIORITY_DEFAULT, self->cancel, on_query_info_ready, NULL);
  }
}


static gboolean
handle_stop_thumbnailing (PtImplThumbnailer *impl, GDBusMethodInvocation *invocation,
                          GVariant *options, gpointer data)
{
  PtApplication *self = data;

  g_debug ("Handling %s", g_dbus_method_invocation_get_method_name (invocation));

  g_queue_clear_full (self->queue, g_object_unref);
  g_cancellable_cancel (self->cancel);
  g_clear_object (&self->cancel);

  pt_impl_thumbnailer_complete_stop_thumbnailing (impl, invocation);

  return TRUE;
}


typedef struct {
  PtImplThumbnailer     *impl;
  GDBusMethodInvocation *invocation;
  GFileEnumerator       *enumerator;
} ThumbnailDirectoryHandle;


static void
thumbnail_directory_handle_free (ThumbnailDirectoryHandle *handle)
{
  g_clear_object (&handle->enumerator);
  g_free (handle);
}


static void
on_enumerator_close_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  ThumbnailDirectoryHandle *handle = data;
  g_autoptr (GError) error = NULL;
  gboolean success;

  success = g_file_enumerator_close_finish (handle->enumerator, result, &error);
  if (!success)
    log_error (error, "Failed to close enumerator: %s", error->message);

  thumbnail_directory_handle_free (handle);
}


static void
on_next_files_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  ThumbnailDirectoryHandle *handle = data;
  g_autoptr (GError) error = NULL;
  g_autolist (GFileInfo) list = NULL;

  list = g_file_enumerator_next_files_finish (handle->enumerator, result, &error);
  if (error) {
    log_error (error, "Failed to enumerate: %s", error->message);
    g_file_enumerator_close_async (handle->enumerator, G_PRIORITY_DEFAULT, self->cancel,
                                   on_enumerator_close_ready, handle);
    g_queue_clear_full (self->queue, g_object_unref);
    return;
  }

  for (GList *head = list; head; head = head->next) {
    GFileInfo *info = head->data;
    GFile *file = g_file_enumerator_get_child (handle->enumerator, info);
    g_queue_push_tail (self->queue, file);
  }

  if (g_list_length (list) != 0) {
    g_file_enumerator_next_files_async (handle->enumerator, 1, G_PRIORITY_DEFAULT, self->cancel,
                                        on_next_files_ready, handle);
  } else {
    g_file_enumerator_close_async (handle->enumerator, G_PRIORITY_DEFAULT, self->cancel,
                                   on_enumerator_close_ready, handle);
    process_queue (CONCURRENCY_LIMIT);
  }
}


static void
on_enumerate_children_ready (GObject *source, GAsyncResult *result, gpointer data)
{
  PtApplication *self = PT_APPLICATION (g_application_get_default ());
  GFile *directory = G_FILE (source);
  ThumbnailDirectoryHandle *handle = data;
  g_autoptr (GError) error = NULL;

  handle->enumerator = g_file_enumerate_children_finish (directory, result, &error);
  if (!handle->enumerator) {
    log_error (error, "Failed to enumerate directory: %s", error->message);
    g_dbus_method_invocation_return_error (handle->invocation, error->domain, error->code, "%s",
                                           error->message);
    thumbnail_directory_handle_free (handle);
    return;
  }

  g_file_enumerator_next_files_async (handle->enumerator, 1, G_PRIORITY_DEFAULT, self->cancel,
                                      on_next_files_ready, handle);

  pt_impl_thumbnailer_complete_thumbnail_directory (handle->impl, handle->invocation);
}


static gboolean
handle_thumbnail_directory (PtImplThumbnailer *impl, GDBusMethodInvocation *invocation,
                            const char *directory, GVariant *options, gpointer data)
{
  PtApplication *self = data;
  g_autoptr (GFile) dir = NULL;
  ThumbnailDirectoryHandle *handle = g_new0 (ThumbnailDirectoryHandle, 1);

  g_debug ("Handling %s: %s", g_dbus_method_invocation_get_method_name (invocation), directory);

  g_queue_clear_full (self->queue, g_object_unref);
  g_cancellable_cancel (self->cancel);
  g_clear_object (&self->cancel);
  self->cancel = g_cancellable_new ();

  handle->impl = impl;
  handle->invocation = invocation;
  dir = g_file_new_for_uri (directory);

  g_file_enumerate_children_async (dir, G_FILE_ATTRIBUTE_STANDARD_NAME,
                                   G_FILE_QUERY_INFO_NONE, G_PRIORITY_DEFAULT, self->cancel,
                                   on_enumerate_children_ready, handle);

  return TRUE;
}


static gboolean
handle_thumbnail_files (PtImplThumbnailer *impl, GDBusMethodInvocation *invocation,
                        const char *const *files, GVariant *options, gpointer data)
{
  PtApplication *self = data;

  g_debug ("Handling %s: %d files", g_dbus_method_invocation_get_method_name (invocation),
           g_strv_length ((GStrv) files));

  g_queue_clear_full (self->queue, g_object_unref);
  g_cancellable_cancel (self->cancel);
  g_clear_object (&self->cancel);
  self->cancel = g_cancellable_new ();

  for (uint i = 0; files[i]; i++) {
    GFile *file = g_file_new_for_uri (files[i]);
    g_queue_push_tail (self->queue, file);
  }

  process_queue (CONCURRENCY_LIMIT);

  pt_impl_thumbnailer_complete_thumbnail_files (impl, invocation);

  return TRUE;
}


static void
pt_application_startup (GApplication *application)
{
  PtApplication *self = PT_APPLICATION (application);
  GnomeDesktopThumbnailSize size = get_thumbnail_size ();

  g_message ("Using thumbnail size %d", size);
  self->factory = gnome_desktop_thumbnail_factory_new (get_thumbnail_size ());
  self->queue = g_queue_new ();

  G_APPLICATION_CLASS (pt_application_parent_class)->startup (application);
}


static gboolean
pt_application_dbus_register (GApplication *application, GDBusConnection *connection,
                              const char *object_path, GError **error)
{
  PtApplication *self = PT_APPLICATION (application);
  GDBusInterfaceSkeleton *interface;

  self->impl = pt_impl_thumbnailer_skeleton_new ();
  interface = G_DBUS_INTERFACE_SKELETON (self->impl);

  g_signal_connect (interface, "handle-thumbnail-files", G_CALLBACK (handle_thumbnail_files), self);
  g_signal_connect (interface, "handle-thumbnail-directory",
                    G_CALLBACK (handle_thumbnail_directory), self);
  g_signal_connect (interface, "handle-stop-thumbnailing", G_CALLBACK (handle_stop_thumbnailing),
                    self);

  return g_dbus_interface_skeleton_export (interface,
                                           connection,
                                           object_path,
                                           error);
}


static void
pt_application_activate (GApplication *application)
{
  PtApplication *self = PT_APPLICATION (application);

  if (!self->hold) {
    g_message ("Activated service");
    g_application_hold (application);
    self->hold = TRUE;
  }
}


static void
pt_application_dispose (GObject *object)
{
  PtApplication *self = PT_APPLICATION (object);

  g_cancellable_cancel (self->cancel);
  g_clear_object (&self->cancel);
  g_clear_object (&self->factory);
  if (self->queue) {
    g_queue_free_full (self->queue, g_object_unref);
    self->queue = NULL;
  }
  g_variant_dict_clear (&self->thumbnails);
}


static void
pt_application_class_init (PtApplicationClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);
  GApplicationClass *application_class = G_APPLICATION_CLASS (klass);

  object_class->dispose = pt_application_dispose;
  application_class->activate = pt_application_activate;
  application_class->dbus_register = pt_application_dbus_register;
  application_class->startup = pt_application_startup;
}


static void
message_handler (const char *domain, GLogLevelFlags level, const char *message, void *data)
{
  GLogLevelFlags new_level = level;

  if (level & G_LOG_LEVEL_DEBUG &&
      g_strstr_len (domain, strlen (G_LOG_DOMAIN), G_LOG_DOMAIN)) {
    new_level &= ~G_LOG_LEVEL_DEBUG;
    new_level |= G_LOG_LEVEL_MESSAGE;
  }

  g_log_default_handler (domain, new_level, message, data);
}


static int
on_handle_local_options (PtApplication *self, GVariantDict *options)
{
  gboolean verbose;

  if (g_variant_dict_lookup (options, "verbose", "b", &verbose)) {
    g_message ("Using verbose logging");
    g_log_set_default_handler (message_handler, NULL);
  }

  return -1;
}


static gboolean
on_shutdown_signal (gpointer data)
{
  PtApplication *self = data;

  g_message ("Exiting gracefully");
  g_application_release (G_APPLICATION (self));

  return G_SOURCE_REMOVE;
}


static void
pt_application_init (PtApplication *self)
{
  g_application_set_option_context_summary (G_APPLICATION (self),
                                            "A service to thumbnail directories.");
  g_application_set_option_context_description (G_APPLICATION (self),
                                                "This utility provides a D-Bus service to "
                                                "thumbnail all supported files in a directory.\n\n"
                                                "Please report issues at https://gitlab.gnome.org/World/Phosh/xdg-desktop-portal-phosh/-/issues.");
  g_application_add_main_option (G_APPLICATION (self),
                                 "verbose", 'v',
                                 0, G_OPTION_ARG_NONE,
                                 "Print debug information.", NULL);

  g_signal_connect (self, "handle-local-options", G_CALLBACK (on_handle_local_options), NULL);
  g_unix_signal_add (SIGTERM, on_shutdown_signal, self);
  g_unix_signal_add (SIGINT, on_shutdown_signal, self);
}


PtApplication *
pt_application_new (void)
{
  return g_object_new (PT_TYPE_APPLICATION,
                       "application-id", PT_SERVICE_DBUS_NAME,
                       "flags", G_APPLICATION_ALLOW_REPLACEMENT,
                       "version", PT_VERSION,
                       NULL);
}
