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

r = requests.post(url = URL, json=data)

# generate 100 movies
# will 500 internal error the server after the database is populated once. 
for i in range(0, 10):
    data['id'] = i
    data['title'] = sentence.gen(3)
    data['year'] = random.randint(1967, 2023)
    data['description'] = sentence.gen(20) + "."

    r = requests.post(url = URL, json=data)
    print(r.status_code)

# Fetch 100 movies
for i in range(0, 10):
    get_url = "http://127.0.0.1:8000/movies/{}".format(i)
    r = requests.get(url=get_url)
    print(r.status_code, r.json)




# extracting data in json format
# print(r.status_code)
# data = r.json()


# URLget = "http://127.0.0.1:8000/movies/0"
# r = requests.get(url = URLget)
# print(r)