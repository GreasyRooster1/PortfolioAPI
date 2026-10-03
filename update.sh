read -p "Press [Enter] to confirm you want to discard local changes"
git reset --hard
git pull
echo "-" >> VERSION
date +%Y%m%d%H%M%S >> VERSION
cargo run --release
systemctl restart portfolio-api.service
