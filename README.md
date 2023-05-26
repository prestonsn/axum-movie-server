## Usage

First run the axum server:

```
cargo run
```

```
export DATABASE_URL=postgres://localhost/your_db
diesel migration run
cargo run -p example-diesel-async-postgres
```

Then in another terminal run:

```
python3 test.py
```

Sample output:

```
 post() incoming json : Movie { name: "movie_name_A", slug: "a", year: 2021, desc: "This is test movie A" }
 get() incoming json : "a"
```
