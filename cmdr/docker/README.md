# Run the build in docker

```shell
cd cmdr && fish run.fish docker-build
```

# Install docker & docker compose

```shell
sudo apt update
sudo apt install -y docker.io docker-compose
```

# Verify install of docker & docker compose

```shell
sudo docker run hello-world
docker compose version
```

# Run docker without sudo, requires logout

```shell
sudo groupadd docker
sudo usermod -aG docker $USER
gnome-session-quit --logout --force --no-prompt
```

# Enable docker to start at boot

```shell
sudo systemctl enable docker
```

# Uninstall docker

```shell
# Remove Docker Engine
sudo apt purge -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Remove Docker data (be careful, this removes all containers, images, volumes)
sudo rm -rf /var/lib/docker
sudo rm -rf /var/lib/containerd
```