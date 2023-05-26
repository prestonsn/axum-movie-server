import requests
  
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

# extracting data in json format
print(r.status_code)
# data = r.json()


# URLget = "http://127.0.0.1:8000/movies/0"
# r = requests.get(url = URLget)
# print(r)