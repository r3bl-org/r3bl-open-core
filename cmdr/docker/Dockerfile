# Get the base image.
FROM ubuntu:24.04

# This is the working directory in the container.
WORKDIR /app

# Copy the current directory to the container.
COPY install.bash .

# Run the install script.
RUN chmod +x install.bash

CMD ["/bin/bash", "-c", "/app/install.bash"]