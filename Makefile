.PHONY: fix
fix:
	cargo lfix --allow-dirty --allow-staged -q --all-features
	cargo lclippy --fix --allow-dirty --allow-staged --all-features
	cargo fmt --all
	cargo sort -w

.PHONY: migrate
migrate:
	cargo sqlx migrate run


.PHONY: prepare
prepare:
	cargo sqlx prepare --workspace