if get_option('man')
  manpages = [['phosh-thumbnailer', 8], ['ptcli', 1]]

  rst2man = find_program('rst2man', 'rst2man.py', required: false)
  rst2man_flags = ['--syntax-highlight=none']

  foreach manpage : manpages
    man_name = manpage[0]
    man_section = manpage[1]

    custom_target(
      'man-@0@'.format(man_name),
      input: '@0@.rst'.format(man_name),
      output: '@0@.@1@'.format(man_name, man_section),
      command: [rst2man, rst2man_flags, '@INPUT@'],
      capture: true,
      install: true,
      install_dir: get_option('mandir') / 'man@0@'.format(man_section),
    )
  endforeach
endif
