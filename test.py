import requests
  
URL = "http://127.0.0.1:8000/movies"
  
data = {
    'name': "movie_name_A",
    'slug': "a",
    'year': 2021,
    'desc': "This is test movie A"
}

r = requests.post(url = URL, json=data)

# extracting data in json format
print(r.status_code)
# data = r.json()


URLget = "http://127.0.0.1:8000/movies/a"
r = requests.get(url = URLget)
print(r)