import geojson
import json
import folium


from geojson import FeatureCollection, Feature, Point

with open('ip.json', 'r') as file:
    data = json.load(file)

ips = []
for key, ip in data["found"].items():
    print(ip)
    loc = Point((float(ip["longitude"]),float(ip["latitude"])))
    feature = Feature(geometry=loc, properties={"country": ip["country"],"region":ip["region"],"city":ip["city"],"isp":ip["isp"]})
    ips.append(feature)

ip_collection = FeatureCollection(ips)
geojson_data = geojson.dumps(ip_collection, sort_keys=True)
print(geojson_data)

m = folium.Map(location=[37.7749, -122.4194], zoom_start=12)
folium.GeoJson(geojson_data, name='geojson_layer').add_to(m)

m.save('map.html')