"""Small authored MVT fixture (protobuf wire encoding), no downloaded OSM data."""
from pathlib import Path


def varint(n):
    result = bytearray()
    while n > 127:
        result.append((n & 127) | 128)
        n >>= 7
    return bytes(result) + bytes([n])


def field(n, value):
    if isinstance(value, int):
        return varint(n << 3) + varint(value)
    return varint((n << 3) | 2) + varint(len(value)) + value


def packed(values):
    return b''.join(map(varint, values))


def geometry(points, closed=False):
    x = y = 0
    values = []
    for i, (nx, ny) in enumerate(points):
        if i == 0:
            values.append(9)
        elif i == 1:
            values.append(((len(points)-1) << 3) | 2)
        for delta in (nx-x, ny-y):
            values.append((delta << 1) ^ (delta >> 63))
        x, y = nx, ny
    if closed:
        values.append(15)
    return packed(values)


def layer(name, keys, values, points, polygon):
    feature = (field(1, 1) + field(2, packed([0, 0])) + field(3, 3 if polygon else 2)
               + field(4, geometry(points, polygon)))
    result = field(15, 2) + field(1, name.encode()) + field(2, feature)
    for key in keys:
        result += field(3, key.encode())
    for value in values:
        result += field(4, value)
    return field(3, result + field(5, 4096))


building = layer('building', ['render_height'], [field(5, 24)],
                 [(1000, 1000), (2000, 1000), (2000, 2000), (1000, 2000)], True)
road = layer('transportation', ['class'], [field(1, b'primary')],
             [(0, 2500), (4096, 2500)], False)
Path(__file__).with_name('city.pbf').write_bytes(building + road)
