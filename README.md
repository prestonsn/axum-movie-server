# [For an interview]

First run the axum server:

Requires postgres setup first. Then run `source load_env.sh` after adjusting the script.

```
diesel migration run
cargo run -p axum-moviesdb
```

Then in another terminal run:

```
cd tests/
python3 test.py
```
