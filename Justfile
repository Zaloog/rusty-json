setup:
	uv sync --all-extras
	uv run python -m maturin_import_hook site install
