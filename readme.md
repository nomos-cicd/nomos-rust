```bash
docker build -t nomos-rust .

# Local
docker run -p 3000:3000 -e NOMOS_USERNAME=admin -e NOMOS_PASSWORD=admin --name nomos nomos-rust

# Production
docker run -d -v <host_path>:/var/lib/nomos -e NOMOS_USERNAME=<username> -e NOMOS_PASSWORD=<password> -e VIRTUAL_HOST=nomos.requizm.com -e VIRTUAL_PORT=3000 -e LETSENCRYPT_HOST=nomos.requizm.com --network common-network --name nomos --user root nomos-rust
```
