
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
sudo pacman -S nodejs npm git
cd /opt
sudo mkdir /opt/planka
sudo chown $(whoami) /opt/planka
git clone https://github.com/plankanban/planka.git /opt/planka
cd /opt/planka

sudo pacman -S nodejs-concurrently # Undeclared dependency of planka
sudo pacman -S python-distutils-extra # Undeclared dependency of planka

npm install

sudo -u postgres createuser 'user' # create the DB user that planka wants

# Test NPM server w/
npm run server:db:init && npm start --prod

sudo tee /etc/systemd/system/planka.service <<EOF
[Unit]
Description=Planka Server
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=1
User=user
WorkingDirectory=/opt/planka
ExecStart=/bin/sh -c /opt/planka/start.sh
RuntimeMaxSec=1200m
StandardError=journal
StandardOutput=journal
StandardInput=null
TimeoutStopSec=4

[Install]
WantedBy=multi-user.target

EOF
sudo systemctl daemon-reload
sudo systemctl enable --now planka

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





```

# Current TODOs

`npm run server:db:init && npm start --prod` does not actually start the server -_-
