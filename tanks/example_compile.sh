clang-9 -nostdlib --target=wasm32 -Wl,--export-all -Wl,--allow-undefined -Wl,--no-entry -ofire_circle.wasm fire_circle.c
