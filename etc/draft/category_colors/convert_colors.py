#!/usr/bin/env python3

import colorsys
import csv


def parse_hex_color(c: str) -> tuple[int, int, int]:
    return (int(c[1:3], 16), int(c[3:5], 16), int(c[5:7], 16))


with open("bootstrap_colors.csv") as f:
    reader = csv.DictReader(f, delimiter=";")
    data = list(reader)

for row in data:
    print(row["name"])
    base_rgb = parse_hex_color(row["base"])
    base_hls = colorsys.rgb_to_hls(*(v / 255 for v in base_rgb))
    for usage in ("bg dark","text dark","border dark","bg light","text light","border light","base"):
        rgb = parse_hex_color(row[usage])
        hls = colorsys.rgb_to_hls(*(v / 255 for v in rgb))
        if usage == "base":
            #print("base: ", end="")
            pass
        else:
            print(hls[1]/base_hls[1])
        #print([f"{v:0.2f}" for v in hsv])


# hue immer gleich
# saturation ungefähr gleich, ggf. bei starker Lumi-Abweichung verringern (~ halbieren)
# Lumineszenz: ?

for usage, usage_lum in zip(("bg dark","text dark","border dark","bg light","text light","border light","base"),
                            (0.1,      0.7,        0.3,          0.9,       0.2,         0.8,           0.5)):
    print(usage)
    for row in data:
        base_rgb = parse_hex_color(row["base"])
        base_hls = colorsys.rgb_to_hls(*(v / 255 for v in base_rgb))
        rgb = parse_hex_color(row[usage])
        hls = colorsys.rgb_to_hls(*(v / 255 for v in rgb))
        print(hls[1], usage_lum + (base_hls[1]/5-0.1))
