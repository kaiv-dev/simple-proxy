#,linux/arm64
docker-build:
	docker buildx build --platform linux/amd64 -t simple-proxy -f Dockerfile .
docker-rm:
	docker rm -f simple-proxy || true
docker-run: docker-build
	docker run --rm --name simple-proxy -v ./certs:/app/certs -v ./proxy.toml:/app/proxy.toml --env-file .env -p 443:443 simple-proxy
