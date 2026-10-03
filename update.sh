read -p "Press [Enter] to confirm you want to discard local changes"
git reset --hard
git pull
chmod +x update.sh
echo -n "-" >> VERSION
date +%Y%m%d%H%M%S >> VERSION
cargo build --release
systemctl restart portfolio-api.service
