config_h = configuration_data()
config_h.set_quoted('GETTEXT_PACKAGE', 'phosh-mobile-portal')
config_h.set_quoted('LOCALEDIR', localedir)
config_h.set_quoted('PT_VERSION', meson.project_version())
config_h.set_quoted('PT_SERVICE_DBUS_NAME', pt_service_dbus_name)
config_h.set_quoted('PT_SERVICE_OBJECT_PATH', pt_service_object_path)

configure_file(output: 'pt-config.h', configuration: config_h)

pt_interface_files = [meson.project_build_root() / 'data' / '@0@.xml'.format(pt_service_dbus_name)]
pt_generated_sources = gnome.gdbus_codegen(
  'phosh-thumbnailer-service',
  sources: pt_interface_files,
  interface_prefix: 'mobi.phosh',
  namespace: 'PtImpl',
)

pt_service_sources = files('application.c', 'service.c') + pt_generated_sources
pt_service_deps = [gtk_dep, gnome_desktop_dep]
pt_service = executable(
  pt_service_exe_name,
  pt_service_sources,
  dependencies: pt_service_deps,
  install: true,
  install_dir: libexecdir,
)

pt_cli_sources = files('cli.c') + pt_generated_sources
pt_cli_deps = [gio_dep]
pt_cli = executable(
  pt_cli_exe_name,
  pt_cli_sources,
  dependencies: pt_cli_deps,
  install: true,
  install_dir: bindir,
)
