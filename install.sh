#!/bin/bash
BINARY="rust_chat"
if [[ $(id -u) -ne 0 ]] ; then echo "Please run as root" ; exit 1 ; fi

echo "Stopping service..."
systemctl stop dart-chat
echo "Installing..."
if [[ -f "target/release/$BINARY" ]]
then
  cp target/release/$BINARY /usr/bin/$BINARY
else
  echo "The build does not exist."
  exit 1
fi
rm -f /etc/systemd/system/$BINARY.service
cat <<EOT >> /etc/systemd/system/$BINARY.service
[Unit]
Description=Chat by Rust
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/$BINARY
Restart=on-abnormal
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOT
echo "Adding firewall..."
firewall-cmd --permanent --zone=public --add-port=9339/tcp
firewall-cmd --reload
echo "Starting service.."
systemctl enable $BINARY
systemctl start $BINARY
systemctl status $BINARY
