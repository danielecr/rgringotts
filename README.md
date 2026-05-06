# rgringotts: your remote connection to gringotts storage

Clever or idiot, this is it: a service providing access to your remote secret file via an API

## How to

1. The service publish a REST API to access data, via libgringotts.
2. The API is secured by session create, and session session destroy.
3. Session is opened by secret passphrase insertion.
4. Session expires after 30 seconds. A keepalive message is supposed to keep it alive (unsurprisingly)
5. The API is exposed in a given port (default 7979), and it is supposed to be tunneled over ssh channel.

Example usage:

```
rgringotts -p 7978 -h localhost \
  -f mydata=/home/user/.gringotts/main \
  -f archive=/mnt/backup/gringotts
```

Since it is on localhost, ssh tunnel is required to attach it. Be aware: stop the service when you have done.

## Configuration

Settings are read from a TOML file first, then CLI flags override individual values.
Folder mappings from both sources are **merged** (CLI additions win on name conflicts).

### Config file

Default location: `./rgringotts.toml` (auto-loaded if it exists).
Override with `--config /path/to/file.toml`.

```toml
# rgringotts.toml
port = 7979
host = "127.0.0.1"

[folders]
# exposed_name = "/absolute/path/to/directory"
mydata1  = "/home/user/.gringotts/main"
archive  = "/mnt/backup/gringotts"
```

### CLI options

| Flag | Default | Description |
|------|---------|-------------|
| `-c`, `--config FILE` | `./rgringotts.toml` | Path to the TOML config file. |
| `-p`, `--port PORT` | `7979` | TCP port to listen on. |
| `-h`, `--host HOST` | `127.0.0.1` | Address to bind to. |
| `-f`, `--folder NAME=PATH` | — | Add a folder mapping (repeatable). |

### Folder mappings

Each mapping exposes a local directory under an **alias**. Clients use
`alias:///filename` as the file specifier in all API calls, so the server
never exposes raw filesystem paths beyond what you explicitly allow.

When **no** folder mappings are configured the service falls back to accepting
raw absolute paths (useful for local testing).

## Security: USE SSH

There is no security on protocol, so use ssh channel to communicate: http has no ssl. This is by design.


## REST API

All endpoints that operate on an open session require the bearer token returned
by `POST /api/session/open` in the `Authorization` header:

```
Authorization: Bearer <token>
```

### Folder discovery

| Method | Path | Success | Description |
|--------|------|---------|-------------|
| `GET` | `/folders` | `200` `["name1","name2",…]` | List all exposed folder aliases. |
| `GET` | `/folders/{name}` | `200` `["file1.grg",…]` | List files inside a mapped folder. |

### Session lifecycle

| Method | Path | Body | Success | Description |
|--------|------|------|---------|-------------|
| `POST` | `/api/session/open` | `{"file":"name:///filename","passphrase":"…"}` | `201` `{"token":"<uuid>"}` | Decrypt the gringotts file and open a session. |
| `POST` | `/api/session/keepalive` | — | `204` | Reset the 30-second inactivity timer. |
| `DELETE` | `/api/session` | — | `204` | Persist changes back to disk and destroy the session. |

#### File specifier format

```
folder_alias:///filename
```

`folder_alias` is the name defined in `[folders]` (config file) or via
`--folder`.  The triple-slash is conventional URI notation (empty authority,
absolute path within the folder).  Examples:

```
mydata1:///notes.grg
archive:///work/passwords.grg
```

Path traversal (`..`) is rejected by the server.

The session **expires automatically after 30 seconds of inactivity**. Any call
that carries a valid token also resets the timer, so a client only needs an
explicit keepalive when it is idle for longer than 30 seconds.

### Entries

Each entry has an `id` (integer), a `title`, and a `body` (plain text).

| Method | Path | Body | Success | Description |
|--------|------|------|---------|-------------|
| `GET` | `/api/entries` | — | `200` `[{"id":0,"title":"…"},…]` | List all entry titles. |
| `POST` | `/api/entries` | `{"title":"…","body":"…"}` | `201` Entry | Create a new entry. |
| `GET` | `/api/entries/{id}` | — | `200` Entry | Retrieve a single entry (title + body). |
| `PUT` | `/api/entries/{id}` | `{"title":"…","body":"…"}` | `200` Entry | Replace title and body of an existing entry. |
| `DELETE` | `/api/entries/{id}` | — | `204` | Remove an entry. |

#### Entry object

```json
{
  "id": 0,
  "title": "My first page",
  "body": "Some secret text."
}
```

### Error responses

All errors return a plain-text body describing the problem.

| Status | Meaning |
|--------|---------|
| `401 Unauthorized` | Missing, invalid, or expired bearer token; wrong passphrase. |
| `404 Not Found` | The requested entry id does not exist. |
| `500 Internal Server Error` | Unexpected failure (e.g. disk I/O). |

### Example session (curl)

```bash
# 1. Discover available folders and files
curl -s http://localhost:7979/folders | jq
curl -s http://localhost:7979/folders/mydata1 | jq

# 2. Open a session using a folder-mapped file
TOKEN=$(curl -s -X POST http://localhost:7979/api/session/open \
  -H 'Content-Type: application/json' \
  -d '{"file":"mydata1:///notes.grg","passphrase":"s3cr3t"}' \
  | jq -r .token)

# 3. List entries
curl -s http://localhost:7979/api/entries \
  -H "Authorization: Bearer $TOKEN" | jq

# 4. Read entry 0
curl -s http://localhost:7979/api/entries/0 \
  -H "Authorization: Bearer $TOKEN" | jq

# 5. Create an entry
curl -s -X POST http://localhost:7979/api/entries \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Bank PIN","body":"1234"}' | jq

# 6. Update it (id returned by the create call, e.g. 1)
curl -s -X PUT http://localhost:7979/api/entries/1 \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Bank PIN","body":"5678"}' | jq

# 7. Keep the session alive while idle
curl -s -X POST http://localhost:7979/api/session/keepalive \
  -H "Authorization: Bearer $TOKEN"

# 8. Save and close
curl -s -X DELETE http://localhost:7979/api/session \
  -H "Authorization: Bearer $TOKEN"
```


## Gringotts and libgringotts

Gringotts is a secure notes manager for Linux and UNIX-like systems, while libgringotts is the underlying C library that handles its data encapsulation and encryption. They are designed to store sensitive information like passwords and credit card numbers securely. [1, 2, 3] 
## Gringotts (The Application)

* Purpose: A graphical application (GTK-based) to store sensitive data in an organized, encrypted, and compressed format.
* Status: Originally created by Germano Rizzo, it is currently maintained by Shlomi Fish.
* Features: It focuses on security and is tailored for Unix-like operating systems. [2, 3] 

## libgringotts (The Library)

* Purpose: A small, thread-safe C library that handles the heavy lifting of encrypting and compressing the data.
* Encryption Algorithms: It uses strong algorithms, including Rijndael (AES) 128/256, Serpent, Twofish, CAST256, SAFER+, LOKI97, and 3DES.
* Hashing Algorithms: Uses SHA1 and RIPEMD-160.
* Compression: Supports ZLib and BZip2.
* Usage: It can be used independently to develop other applications requiring secure storage, as seen in the [LibGringotts - Free Software Directory](https://directory.fsf.org/project/libGringotts/) and the [libgringotts GitLab repository](https://gitlab.com/deb-pkg/libgringotts). [4, 5, 6] 

## Resources and Packages

* Homepage: [Gringotts - a Safebox for your Data](https://gringotts.shlomifish.org/).
* Linux Packages: The library is available in repositories for Debian (see the [Debian Package Search Results for gringotts](https://packages.debian.org/gringotts)), Fedora, and openSUSE.
* SourceForge: [Gringotts download page](https://sourceforge.net/projects/gringotts.berlios/). [1, 2, 4, 7, 8] 

The software is considered quite old by modern standards, as noted in developer discussions, but remains functional. [9] 

[1] [https://sourceforge.net](https://sourceforge.net/projects/gringotts.berlios/)
[2] https://gringotts.shlomifish.org
[3] [https://packages.altlinux.org](https://packages.altlinux.org/en/p11/srpms/gringotts/)
[4] [https://packages.fedoraproject.org](https://packages.fedoraproject.org/pkgs/libgringotts/libgringotts/)
[5] [https://directory.fsf.org](https://directory.fsf.org/project/libGringotts/)
[6] [https://gitlab.com](https://gitlab.com/deb-pkg/libgringotts#:~:text=*%20Highly%20customizable%20&%20easy%20to%20use,same%20strong%20encryption%20of%20the%20data%20files.)
[7] [https://software.opensuse.org](https://software.opensuse.org/package/libgringotts)
[8] [https://packages.debian.org](https://packages.debian.org/gringotts)
[9] [https://github.com](https://github.com/void-linux/void-packages/issues/21359)
