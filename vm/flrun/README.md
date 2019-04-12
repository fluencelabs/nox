# How to run

Given a file `hello_world.wasm`:
```bash
docker run --rm -v hello_world.wasm:/code/llama_db.wasm -p 30000:30000 fluencelabs/frun
```
