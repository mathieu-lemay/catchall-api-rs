IMAGE_NAME := "acidrain/catchall-api-actix"

run:
    cargo-run

lint:
    pre-commit run --all-files

test:
    cargo test

docker-build:
    docker build --ssh default --tag "{{ IMAGE_NAME }}:$(git describe --always HEAD)" .

docker-run: docker-build
    docker run --rm -it -p 8080:8080 --name catchall-api-actix "{{ IMAGE_NAME }}:$(git describe --always HEAD)"

docker-push: docker-build
    docker push "{{ IMAGE_NAME }}:$(git describe --always HEAD)"

@docker-tag:
    git describe --always HEAD
