IMAGE_NAME = wes-sfu
CONTAINER_NAME = wes-sfu-container

.PHONY: all build up down clean shell logs

# Default target
all: build

# Build the Docker image
build:
	docker build -t $(IMAGE_NAME) .

# Run the container in detached mode with a name
up:
	docker run -d -p 8069:8069/tcp -p 8070:8070/udp --name $(CONTAINER_NAME) $(IMAGE_NAME)

# Stop the running container
down:
	docker stop $(CONTAINER_NAME)

# Remove container and image
clean:
	docker rm -f $(CONTAINER_NAME)
	docker rmi $(IMAGE_NAME)

# Start an interactive shell inside the running container
shell:
	docker exec -it $(CONTAINER_NAME) bash

# Show logs from the container
logs:
	docker logs $(CONTAINER_NAME)
