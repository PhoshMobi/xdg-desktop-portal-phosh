i18n = import('i18n')

data_conf = configuration_data()
data_conf.set('libexecdir', libexecdir)
data_conf.set('dbusname', pmp_dbus_name)
data_conf.set('executablename', pmp_exe_name)
data_conf.set('portals', ';'.join(pmp_portals))

phrosh_conf = configuration_data()
phrosh_conf.set('DBUS_NAME', phrosh_dbus_name)
phrosh_conf.set('EXE_NAME', phrosh_exe_name)
phrosh_conf.set('INTERFACES', ';'.join(phrosh_interfaces))
phrosh_conf.set('LIBEXEC_DIR', libexecdir)

pt_conf = configuration_data()
pt_conf.set('DBUS_NAME', pt_service_dbus_name)
pt_conf.set('INTERFACE_NAME', pt_service_dbus_name)
pt_conf.set('OBJECT_PATH', pt_service_object_path)
pt_conf.set('EXE_NAME', pt_service_exe_name)
pt_conf.set('LIBEXEC_DIR', libexecdir)

# DBus service file
configure_file(
  input: '@0@.service.in'.format(pmp_dbus_name),
  output: '@0@.service'.format(pmp_dbus_name),
  configuration: data_conf,
  install_dir: servicedir,
)

if get_option('rust-portal')
  configure_file(
    input: '@0@.service.in'.format(phrosh_dbus_name),
    output: '@0@.service'.format(phrosh_dbus_name),
    configuration: phrosh_conf,
    install: true,
    install_dir: servicedir,
  )
endif

configure_file(
  input: '@0@.service.in'.format(pt_service_dbus_name),
  output: '@0@.service'.format(pt_service_dbus_name),
  configuration: pt_conf,
  install: true,
  install_dir: servicedir,
)

# D-Bus interface file
configure_file(
  input: '@0@.xml.in'.format(pt_service_dbus_name),
  output: '@0@.xml'.format(pt_service_dbus_name),
  configuration: pt_conf,
  install: true,
  install_dir: interfacedir,
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

if get_option('rust-portal')
  i18n.merge_file(
    type: 'desktop',
    input: configure_file(
      input: '@0@.desktop.in.in'.format(phrosh_exe_name),
      output: '@0@.desktop.in'.format(phrosh_exe_name),
      configuration: phrosh_conf,
    ),
    output: '@0@.desktop'.format(phrosh_exe_name),
    po_dir: meson.project_source_root() / 'po',
    install: true,
    install_dir: desktopdir,
  )
endif

# Systemd user unit
configure_file(
  input: '@0@.service.in'.format(pmp_exe_name),
  output: '@0@.service'.format(pmp_exe_name),
  configuration: data_conf,
  install_dir: systemd_user_unit_dir,
)

if get_option('rust-portal')
  configure_file(
    input: '@0@.service.in'.format(phrosh_exe_name),
    output: '@0@.service'.format(phrosh_exe_name),
    configuration: phrosh_conf,
    install: true,
    install_dir: systemd_user_unit_dir,
  )
endif

configure_file(
  input: '@0@.service.in'.format(pt_service_exe_name),
  output: '@0@.service'.format(pt_service_exe_name),
  configuration: pt_conf,
  install: true,
  install_dir: systemd_user_unit_dir,
)

# Portal conf
configure_file(
  input: 'phosh.portal.in',
  output: 'phosh.portal',
  configuration: data_conf,
  install_dir: portaldir,
)

if get_option('rust-portal')
  configure_file(
    input: 'phrosh.portal.in',
    output: 'phrosh.portal',
    configuration: phrosh_conf,
    install: true,
    install_dir: portaldir,
  )
endif
