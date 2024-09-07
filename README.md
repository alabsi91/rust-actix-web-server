# serve anything using actix-web

- config.json

```json
{
  "https": {
    "enabled": false,
    "ip": "127.0.0.1",
    "port": 3040,
    "key": "key.pem",
    "cert": "cert.pem"
  },

  "http": {
    "enabled": true,
    "ip": "127.0.0.1",
    "port": 3030
  },

  "file_listing": {
    "enabled": true,
    "dir": "public/files",
    "route": "static"
  },

  "public_dir": "public",
  "not_found_page": "public/404.html",

  "filtering": {
    "ip_whitelist": ["*.*.*.*"],
    "ip_blacklist": [],

    "rate_limit": {
      "per_second": 2,
      "burst_size": 20
    }
  }
}
```
