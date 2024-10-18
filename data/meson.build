i18n = import('i18n')

data_conf = configuration_data()
data_conf.set('libexecdir', libexecdir)
data_conf.set('dbusname', pmp_dbus_name)
data_conf.set('executablename', pmp_exe_name)
data_conf.set('portals', ';'.join(pmp_portals))

# DBus service file
configure_file(
  input: '@0@.service.in'.format(pmp_dbus_name),
  output: '@0@.service'.format(pmp_dbus_name),
  configuration: data_conf,
  install_dir: servicedir,
)

# Desktop file
desktop_in = configure_file(
  input: '@0@.desktop.in.in'.format(pmp_exe_name),
  output: '@0@.desktop.in'.format(pmp_exe_name),
  configuration: data_conf,
)

i18n.merge_file(
  type: 'desktop',
  input: desktop_in,
  output: '@0@.desktop'.format(pmp_exe_name),
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: desktopdir,
)

# Systemd user unit
configure_file(
  input: '@0@.service.in'.format(pmp_exe_name),
  output: '@0@.service'.format(pmp_exe_name),
  configuration: data_conf,
  install_dir: systemd_user_unit_dir,
)

# Portal conf
# Phosh ships a phosh.portal for the Access portal
# so use PhoshMobilePortal to avoid a name class
configure_file(
  input: 'pmp.portal.in',
  output: 'pmp.portal',
  configuration: data_conf,
  install_dir: portaldir,
)
