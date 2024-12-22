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

pacstrap -K /mnt base linux linux-firmware

genfstab -U /mnt >> /mnt/etc/fstab

arch-chroot /mnt

ln -sf /usr/share/zoneinfo/America/New_York /etc/localtime

vim /etc/locale.gen
locale-gen

vim /etc/locale.conf # LANG=en_US.UTF-8

vim /etc/hostname # stitch

mkinitcpio -P

bootctl install




```


