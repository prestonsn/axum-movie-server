import requests
import random
import sentence
  
URL = "http://127.0.0.1:8000/movies"
  
#[derive(Serialize, Deserialize, Debug, Clone, Default, Insertable, Selectable, Queryable)]
#[diesel(table_name = schema::movies)]
# struct Movie {
#     id: i32,
#     title: String,
#     year: i32,
#     description: String,
# }
data = {
    'id': 0,
    'title': "Test Movie A",
    'year': 2021,
    'description': "This is test movie A"
}

# r = requests.post(url = URL, json=data)

# generate 100 movies
# will 500 internal error the server after the database is populated once. 
for i in range(123, 202):
    data['id'] = i
    data['title'] = sentence.gen(3)
    data['year'] = random.randint(1967, 2023)
    data['description'] = sentence.gen(20) + "."

    r = requests.post(url = URL, json=data)
    print(r.status_code)

# Fetch 100 movies.
avg_resp_time = 0
idx_start = 123
idx_end = 201
for i in range(idx_start, idx_end):
    get_url = "http://127.0.0.1:8000/movies/{}".format(i)
    r = requests.get(url=get_url)
    avg_resp_time += r.elapsed.microseconds
    # print("\t{}".format(r.json()))

print("---------------------")
print("Avg Response Time (cold cache): {} µs".format(float(avg_resp_time) / (idx_end - idx_start)))
print("---------------------")

# Fetch them again, but should be cached.
avg_resp_time = 0
idx_start = 123
idx_end = 201
for i in range(idx_start, idx_end):
    get_url = "http://127.0.0.1:8000/movies/{}".format(i)
    r = requests.get(url=get_url)
    avg_resp_time += r.elapsed.microseconds
    # print("\t{}".format(r.json()))

print("---------------------")
print("Avg Response Time (warm cache): {} µs".format(float(avg_resp_time) / (idx_end - idx_start)))
print("---------------------")