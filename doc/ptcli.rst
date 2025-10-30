=====
ptcli
=====

---------------------------------------------
A CLI to interact with ``phosh-thumbnailer``.
---------------------------------------------

SYNOPSIS
--------

``ptcli [OPTION…] /path/to/directory``

``ptcli [OPTION…] /path/to/file_1 /path/to/file_2 …``

``ptcli [OPTION…] --stop``

DESCRIPTION
-----------

``ptcli`` can be used to interact with ``phosh-thumbnailer``\ (8) service. It is primarily used to
create thumbnails as per `Thumbnail Managing Standard
<https://specifications.freedesktop.org/thumbnail-spec/latest/>`_.

``-h``, ``--help``
  Print help options and exit.

``-s``, ``--stop``
  Stop the on-going thumbnailing operation.

``-v``, ``--version``
  Print version and exit.

EXAMPLES
--------

To create thumbnails for all files in the current directory::

  ptcli .

To create thumbnails for all files in ``$HOME/Pictures``::

  ptcli $HOME/Pictures

To create thumbnails for a set of files::

  ptcli $HOME/Pictures/foo.png bar.txt ../spam.ogg

To stop the on-going thumbnailing operation::

  ptcli --stop

SEE ALSO
--------

``phosh-thumbnailer``\ (8)
