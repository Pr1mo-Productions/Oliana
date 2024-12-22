# `stitch` setup notes

`stitch` is another server, setup to perform LLM/AI work for lower-power machines.

See `cloud-infra-access/remote-to-stitch.py` for an interactive setup guide to SSH access.


## Setup Notes

```bash
# (-1) Wifi setup on Live USB
ip link set wlan0 up
iw dev wlan0 scan
iw dev wlan0 connect your_essid
iw dev wlan0 set power_save off
# systemctl start dhcpcd@wlan0.service
sudo dhcpcd -4 wlan0
dmesg -n 1
# Potentially useful
ip address add ADDRESS/24 broadcast + dev wlan0
ip route replace default via ADDRESS dev wlan0
systemctl start systemd-resolved
resolvectl dns wlan0 1.1.1.1 8.8.8.8
resolvectl query archlinux.org

# (0) Install Arch on a PC
fdisk
mkfs.fat -F 32 /dev/nvme0n1p1
bcachefs format /dev/nvme0n1p2
mount /dev/nvme0n1p2 /mnt && mkdir -p /mnt/boot && mount /dev/nvme0n1p1 /mnt/boot

pacman-key --init
pacstrap -K /mnt base linux linux-firmware git base-devel openssh sudo vim amd-ucode

genfstab -U /mnt >> /mnt/etc/fstab

arch-chroot /mnt

ln -sf /usr/share/zoneinfo/America/New_York /etc/localtime

vim /etc/locale.gen
locale-gen

vim /etc/locale.conf # LANG=en_US.UTF-8

vim /etc/hostname # stitch

mkinitcpio -P

bootctl install

vim /boot/loader/entries/stitch.conf <<EOF
title Stitch
linux /vmlinuz-linux
initrd /amd-ucode.img
initrd /initramfs-linux.img
options root=PARTUUID=<output of (lsblk -no NAME,PARTUUID /dev/nvme1n1p2)> rootfstype=bcachefs add_efi_memmap mitigations=off pti=off
EOF
vim /boot/loader/loader.conf <<EOF
console-mode max
timeout 4
default stitch.conf
EOF

bootctl install

# Setup user account
useradd -m -G wheel user
passwd user # See 'Infrastructure Secrets' in Drive folder

mkdir -p /opt/automatics/
vim /opt/automatics/wifi.sh <<EOF
#!/bin/bash

D=wlan0

ip link set $D up
iwctl station $D connect your_essid
iw dev $D set power_save off

sleep 12

if ! ip a | grep -qi 192.168.XX ; then
  # DHCP doesn't work, let's just throw a static address and route on there
  ip address add ADDRESS/24 broadcast + dev $D
  ip route replace default via ADDRESS dev $D
fi

EOF

vim /etc/systemd/system/automatics-wifi.service <<EOF
[Unit]
Description=Bootup WiFi watchdog

[Service]
Type=simple
ExecStart=/bin/bash /opt/automatics/wifi.sh

[Install]
WantedBy=multi-user.target
EOF

systemctl enable automatics-wifi.service

vim /etc/systemd/system/automatics-wifi.timer <<EOF
[Unit]
Description=Bootup WiFi watchdog timer

[Timer]
OnUnitActiveSec=90s
OnBootSec=14s

[Install]
WantedBy=timers.target
EOF

systemctl enable automatics-wifi.timer

pacman -Syu iwd iw amd-ucode

bootctl install

systemctl enable sshd


```

## First Boot + Setup

```bash

# Install yay just because AUR is awesome
sudo pacman -S --needed git base-devel
cd /opt
sudo mkdir /opt/yay
sudo chown user:user /opt/yay
git clone https://aur.archlinux.org/yay.git /opt/yay
cd /opt/yay
makepkg -si

# Install USB4 + Nvidia drivers for thunderbolt access
yay -S wget nvidia-open nvidia-utils opencl-nvidia cuda libtorch-cxx11abi-cuda

vim /etc/udev/rules.d/99-removable.rules <<EOF
ACTION=="add", SUBSYSTEM=="thunderbolt", ATTR{authorized}=="0", ATTR{authorized}="1"
EOF

yay -S bolt
boltctl list
boltctl authorize DEVICE


lspci -k -d ::03xx
# Should dump Nvidia card attached over thunderbolt details

# Firmware stuff
yay -S fwupd
sudo fwupdmgr get-devices
sudo fwupdmgr refresh
sudo fwupdmgr get-updates
sudo fwupdmgr update

yay -S usbutils

yay -S tbtools


```
