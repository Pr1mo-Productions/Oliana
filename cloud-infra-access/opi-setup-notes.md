
# `opi` setup notes

`opi` is a small Rasperry Pi Model B+ that hosts some of our infrastructure!

See `cloud-infra-access/remote-to-opi.py` for an interactive setup guide to SSH access.

## Setup Notes

```bash
# (0) Install Arch on a rasperry pi like so - https://archlinuxarm.org/platforms/armv8/broadcom/raspberry-pi-3

# Rename Default user
usermod -d /home/user -m alarm
usermod -l user alarm

# Install good stuff
pacman -S base-devel git htop

# Postgres is a dependency of Planka
sudo pacman -S postgresql
sudo -u postgres initdb -D /var/lib/postgres/data
sudo systemctl enable --now postgresql

# Install Planka - https://www.ipv6.rs/tutorial/Arch_Linux/Planka/
#sudo pacman -S nodejs npm git
#cd /opt
#sudo mkdir /opt/planka
#sudo chown $(whoami) /opt/planka
#git clone https://github.com/plankanban/planka.git /opt/planka
#cd /opt/planka
#
#sudo pacman -S nodejs-concurrently # Undeclared dependency of planka
#sudo pacman -S python-distutils-extra # Undeclared dependency of planka
#
#npm install
#
#sudo -u postgres createuser 'user' # create the DB user that planka wants
#
## Test NPM server w/
#npm run server:db:init && npm start --prod
#
#sudo tee /etc/systemd/system/planka.service <<EOF
#[Unit]
#Description=Planka Server
#StartLimitIntervalSec=0
#
#[Service]
#Type=simple
#Restart=always
#RestartSec=1
#User=user
#WorkingDirectory=/opt/planka
#ExecStart=/bin/sh -c /opt/planka/start.sh
#RuntimeMaxSec=1200m
#StandardError=journal
#StandardOutput=journal
#StandardInput=null
#TimeoutStopSec=4
#
#[Install]
#WantedBy=multi-user.target
#
#EOF
#sudo systemctl daemon-reload
#sudo systemctl enable --now planka

# Setup some Swap space
sudo mkswap -U clear --size 2G --file /swapfile
vim /etc/fstab <<EOF
/swapfile none swap defaults 0 0
EOF


# Install an AUR helper

sudo mkdir /opt/yay
sudo chown $(whoami) /opt/yay
git clone https://aur.archlinux.org/yay.git /opt/yay
cd /opt/yay
makepkg -si


# Setup a service to regularly update a DNS record
yay -S ddclient
sudo vim /etc/ddclient/ddclient.conf # put in hostname & auth credentials for opi.jmcateer.com
sudo tee /etc/systemd/system/ddclient.service <<EOF
[Unit]
Description=Dynamic DNS updater
Wants=network-online.target
After=network-online.target nss-lookup.target

[Service]
Type=exec
Environment=daemon_interval=10m
ExecStart=/usr/bin/ddclient --daemon \${daemon_interval} --foreground
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
sudo systemctl daemon-reload
sudo systemctl enable --now ddclient

# Create a service to run various commands in /on_boot.sh on boot

sudo vim /on_boot.sh <<EOF
#!/bin/bash

mount -n -o remount,rw /
swapon /swapfile

EOF
sudo tee /etc/systemd/system/on_boot.service <<EOF
[Unit]
Description=Planka Server
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=on-failure
RestartSec=1
User=root
WorkingDirectory=/
ExecStart=/bin/sh -c /on_boot.sh
RuntimeMaxSec=1200m
StandardError=journal
StandardOutput=journal
StandardInput=null
TimeoutStopSec=4

[Install]
WantedBy=multi-user.target

EOF
sudo systemctl daemon-reload
sudo systemctl enable --now on_boot

# Attempt to setup nextcloud for team docs + nextcloud-app-deck, which provided a kanban capability
yay -S nextcloud



```

# Current TODOs

`npm run server:db:init && npm start --prod` does not actually start the server -_-
