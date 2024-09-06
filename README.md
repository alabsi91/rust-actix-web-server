# serve anything using actix-web

- config.json

```json
{
  "https": true,
  "https_port": 3040,
  "openssl_key": "key.pem",
  "openssl_cert": "cert.pem",
  "ip": "127.0.0.1",
  "port": 3030,
  "serve_dir": "public",
  "file_listing_dir": "public/files",
  "file_listing_entry": "static",
  "not_found_file": "public/404.html",
  "blacklist": [],
  "rate_limit": {
    "per_second": 2,
    "burst_size": 5
  }
}
```
