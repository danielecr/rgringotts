# To Serve or Not To Serve

This is not supposed to be an exposed service. It should be executed as binary in the target machine, then the port exposed through ssh tunnel. This is the only way transport can be secured.

A service must mimic all this, including the tunnel. Say by exposing ssh service, accepting ssh client connection, authenticating the user by keypair, opening a tunnel connection to the actual rgringotts service.
But this is far too much complex.

## The great security hole: transport

By itself, the idea to provide a REST interface to gringotts files is a break of the intent of gringotts project. This is like doing something and undoing it because of idiot idea.

But I need something quick to break things.

So I supposed that passing data over an SSH tunnel would not be a terrible idea.

## Client security: the worst part

Still there is problem with the client using these datas.

Gringotts lock data into memory, so it is secured and not accessible.

But the tauri client does nothing like that, it is in fact very relaxed

## Server security

Still, server rgringotts, does not secure data into memory this security is provided by libgringotts, but there is no reinforcement outside it, like locking memory.