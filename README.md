serverize
=========

Turn any (interactive) shell command into a server. Listens on a port and runs the command given in argument for each incoming connection, with the socket as its stdin/stdout.

Warning
=======

Do not use with a shell as the command to be run, or you have an authentication-less telnetd.
