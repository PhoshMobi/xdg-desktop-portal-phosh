/*
 * Copyright © 2019 Red Hat, Inc
 *             2023-2025 The Phosh Developers
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Heavily based on the xdg-desktop-portal-gnome
 *
 * Authors:
 *       Felipe Borges <feborges@redhat.com>
 *       Guido Günther <agx@sigxcpu.org>
 */

#include "pmp-config.h"

#include <string.h>

#include <gdk-pixbuf/gdk-pixbuf.h>
#include <gio/gio.h>
#include <glib/gi18n.h>
#include <gtk/gtk.h>

#define GNOME_DESKTOP_USE_UNSTABLE_API
#include <gnome-bg/gnome-bg.h>

#include "pmp-wallpaper-preview.h"

enum {
  PROP_0,
  PROP_LOCKSCREEN,
  PROP_LAST_PROP
};
static GParamSpec *props[PROP_LAST_PROP];


struct _PmpWallpaperPreview {
  GtkBox                        parent;

  GtkStack                     *stack;
  GtkWidget                    *desktop_preview;
  GtkWidget                    *animated_background_icon;
  GtkLabel                     *desktop_clock_label;
  GtkLabel                     *lockscreen_clock_label;
  GtkLabel                     *lockscreen_date_label;
  GtkWidget                    *drawing_area;

  GnomeDesktopThumbnailFactory *thumbnail_factory;
  GnomeBG                      *bg;

  GSettings                    *desktop_settings;
  gboolean                      is_24h_format;
  GDateTime                    *previous_time;
  guint                         clock_time_timeout_id;
  gboolean                      lockscreen;
};

G_DEFINE_FINAL_TYPE (PmpWallpaperPreview, pmp_wallpaper_preview, GTK_TYPE_BOX)


static void
set_lockscreen (PmpWallpaperPreview *self, gboolean lockscreen)
{
  self->lockscreen = lockscreen;

  if (self->lockscreen)
    gtk_widget_add_css_class (GTK_WIDGET (self), "lockscreen");
  else
    gtk_widget_remove_css_class (GTK_WIDGET (self), "lockscreen");
}


static void
pmp_wallpaper_preview_set_property (GObject      *object,
                                    guint         property_id,
                                    const GValue *value,
                                    GParamSpec   *pspec)
{
  PmpWallpaperPreview *self = PMP_WALLPAPER_PREVIEW (object);

  switch (property_id) {
  case PROP_LOCKSCREEN:
    set_lockscreen (self, g_value_get_boolean (value));
    break;
  default:
    G_OBJECT_WARN_INVALID_PROPERTY_ID (object, property_id, pspec);
    break;
  }
}


static void
pmp_wallpaper_preview_get_property (GObject    *object,
                                    guint       property_id,
                                    GValue     *value,
                                    GParamSpec *pspec)
{
  PmpWallpaperPreview *self = PMP_WALLPAPER_PREVIEW (object);

  switch (property_id) {
  case PROP_LOCKSCREEN:
    g_value_set_boolean (value, self->lockscreen);
    break;
  default:
    G_OBJECT_WARN_INVALID_PROPERTY_ID (object, property_id, pspec);
    break;
  }
}


static void
draw_preview_func (GtkDrawingArea *drawing_area,
                   cairo_t        *cr,
                   int             width,
                   int             height,
                   gpointer        data)
{
  PmpWallpaperPreview *self = PMP_WALLPAPER_PREVIEW (data);
  g_autoptr (GdkMonitor) monitor = NULL;
  g_autoptr (GdkPixbuf) pixbuf = NULL;
  GdkRectangle monitor_layout;
  GdkDisplay *display;
  GListModel *monitors;

  display = gtk_widget_get_display (GTK_WIDGET (drawing_area));
  monitors = gdk_display_get_monitors (display);
  monitor = g_list_model_get_item (monitors, 0);
  gdk_monitor_get_geometry (monitor, &monitor_layout);

  pixbuf = gnome_bg_create_thumbnail (self->bg,
                                      self->thumbnail_factory,
                                      &monitor_layout,
                                      width,
                                      height);
  gdk_cairo_set_source_pixbuf (cr, pixbuf, 0, 0);
  cairo_paint (cr);
}

static void
update_clock_label (PmpWallpaperPreview *self,
                    gboolean             force)
{
  g_autoptr (GDateTime) now = NULL;
  g_autofree gchar *clock_label = NULL;
  g_autofree gchar *date_label = NULL;

  now = g_date_time_new_now_local ();

  if (!force && self->previous_time &&
      g_date_time_get_hour (now) == g_date_time_get_hour (self->previous_time) &&
      g_date_time_get_minute (now) == g_date_time_get_minute (self->previous_time))
    return;

  if (self->is_24h_format)
    clock_label = g_date_time_format (now, "%R");
  else
    clock_label = g_date_time_format (now, "%I:%M %p");

  gtk_label_set_label (self->desktop_clock_label, clock_label);
  gtk_label_set_label (self->lockscreen_clock_label, clock_label);

  date_label = g_date_time_format (now, "%x");
  gtk_label_set_label (self->lockscreen_date_label, date_label);

  g_clear_pointer (&self->previous_time, g_date_time_unref);
  self->previous_time = g_steal_pointer (&now);
}

static void
update_clock_format (PmpWallpaperPreview *self)
{
  g_autofree gchar *clock_format = NULL;
  gboolean is_24h_format;

  clock_format = g_settings_get_string (self->desktop_settings, "clock-format");
  is_24h_format = g_strcmp0 (clock_format, "24h") == 0;

  if (is_24h_format != self->is_24h_format) {
    self->is_24h_format = is_24h_format;
    update_clock_label (self, TRUE);
  }
}

static gboolean
update_clock_cb (gpointer data)
{
  PmpWallpaperPreview *self = PMP_WALLPAPER_PREVIEW (data);

  update_clock_label (self, FALSE);

  return G_SOURCE_CONTINUE;
}

static void
pmp_wallpaper_preview_finalize (GObject *object)
{
  PmpWallpaperPreview *self = PMP_WALLPAPER_PREVIEW (object);

  g_clear_object (&self->desktop_settings);
  g_clear_object (&self->thumbnail_factory);

  g_clear_pointer (&self->previous_time, g_date_time_unref);

  if (self->clock_time_timeout_id > 0) {
    g_source_remove (self->clock_time_timeout_id);
    self->clock_time_timeout_id = 0;
  }

  G_OBJECT_CLASS (pmp_wallpaper_preview_parent_class)->finalize (object);
}

static void
pmp_wallpaper_preview_init (PmpWallpaperPreview *self)
{
  gtk_widget_init_template (GTK_WIDGET (self));

  self->desktop_settings = g_settings_new ("org.gnome.desktop.interface");

  g_signal_connect_object (self->desktop_settings,
                           "changed::clock-format",
                           G_CALLBACK (update_clock_format),
                           self,
                           G_CONNECT_SWAPPED);
  update_clock_format (self);

  self->clock_time_timeout_id = g_timeout_add_seconds (1, update_clock_cb, self);

  self->bg = gnome_bg_new ();
  gnome_bg_set_placement (self->bg, G_DESKTOP_BACKGROUND_STYLE_ZOOM);

  self->thumbnail_factory =
    gnome_desktop_thumbnail_factory_new (GNOME_DESKTOP_THUMBNAIL_SIZE_NORMAL);
}

static void
pmp_wallpaper_preview_class_init (PmpWallpaperPreviewClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);
  GtkWidgetClass *widget_class = GTK_WIDGET_CLASS (klass);
  g_autoptr (GtkCssProvider) provider = gtk_css_provider_new ();

  object_class->finalize = pmp_wallpaper_preview_finalize;
  object_class->get_property = pmp_wallpaper_preview_get_property;
  object_class->set_property = pmp_wallpaper_preview_set_property;

  props[PROP_LOCKSCREEN] =
    g_param_spec_boolean ("lockscreen", "", "",
                          FALSE,
                          G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);

  g_object_class_install_properties (object_class, PROP_LAST_PROP, props);

  gtk_widget_class_set_template_from_resource (widget_class,
                                               "/mobi/phosh/portal/pmp-wallpaper-preview.ui");

  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, stack);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, desktop_preview);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, animated_background_icon);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, drawing_area);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, desktop_clock_label);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, lockscreen_clock_label);
  gtk_widget_class_bind_template_child (widget_class, PmpWallpaperPreview, lockscreen_date_label);


  gtk_css_provider_load_from_resource (provider, "/mobi/phosh/portal/pmp-wallpaper-preview.css");
  gtk_style_context_add_provider_for_display (gdk_display_get_default (),
                                              GTK_STYLE_PROVIDER (provider),
                                              GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
}

PmpWallpaperPreview *
pmp_wallpaper_preview_new (void)
{
  return g_object_new (PMP_TYPE_WALLPAPER_PREVIEW, NULL);
}

void
pmp_wallpaper_preview_set_image (PmpWallpaperPreview *self,
                                 const gchar         *image_uri)
{
  g_autofree char *path = NULL;
  g_autoptr (GFile) image_file = NULL;

  image_file = g_file_new_for_uri (image_uri);
  path = g_file_get_path (image_file);
  gnome_bg_set_filename (self->bg, path);

  gtk_widget_set_visible (self->animated_background_icon,
                          gnome_bg_changes_with_time (self->bg));
  gtk_stack_set_visible_child (GTK_STACK (self->stack), self->desktop_preview);

  gtk_drawing_area_set_draw_func (GTK_DRAWING_AREA (self->drawing_area),
                                  draw_preview_func,
                                  self,
                                  NULL);
  gtk_widget_queue_draw (self->drawing_area);
}
