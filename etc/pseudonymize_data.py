#!/usr/bin/env python
import json
import re
import sys
import random
from typing import TypeAlias

JSON: TypeAlias = dict[str, "JSON"] | list["JSON"] | str | int | float | bool | None


def main():
    with open(sys.argv[1]) as f:
        data = json.load(f)

    with open(sys.argv[3]) as f:
        replacement_types = json.load(f)

    with open("familynames.txt") as f:
        nachnamen = [n.strip() for n in f.readlines()]
    with open("nicknames.txt") as f:
        planets = [n.strip() for n in f.readlines()]
    with open("forenames.txt") as f:
        vornamen = [n.strip() for n in f.readlines()]
    replacement_map = {
        name: random.choice(nachnamen) if t == "n" else random.choice(vornamen) if t == "v" else random.choice(planets)
        for name, t in replacement_types.items()
    }

    unchanged_names = set()
    changed_texts = []
    for entry in data["entries"]:
        anonymize_entry(entry, replacement_map, unchanged_names, changed_texts)

    print("Unchanged names:")
    for name in unchanged_names:
        print(f"    {name}")
    for old_text, new_text in changed_texts:
        print(f"\n<<<<<<\n{old_text}======\n{new_text}<<<<<<")

    with open(sys.argv[2], "w") as f:
        json.dump(data, f, indent=2)


def anonymize_entry(entry: dict[str, JSON], replacement_map: dict[str, str], unchanged_names: set[str],
                    changed_texts: list[tuple[str, str]]):
    regex = re.compile("\\b(" + "|".join(replacement_map.keys()) + ")(s?)\\b")
    new_title = regex.sub(lambda m: replacement_map.get(m.group(1), ""), entry["title"])
    if new_title != entry["title"]:
        changed_texts.append((entry["title"], new_title))
        entry["title"] = new_title
    if "description" in entry:
        new_description = regex.sub(lambda m: replacement_map.get(m.group(1), "") + m.group(2), entry["description"])
        if new_description != entry["description"]:
            changed_texts.append((entry["description"], new_description))
            entry["description"] = new_description
    if "comment" in entry:
        new_comment = regex.sub(lambda m: replacement_map.get(m.group(1), "") + m.group(2), entry["comment"])
        if new_comment != entry["comment"]:
            changed_texts.append((entry["comment"], new_comment))
            entry["comment"] = new_comment
    old_person = entry["responsiblePerson"]
    new_person = regex.sub(lambda m: replacement_map.get(m.group(1), ""), old_person)
    entry["responsiblePerson"] = new_person
    old_names = set(name for name in re.split(r"[, ()]|und", old_person) if name)
    new_names = set(name for name in re.split(r"[, ()]|und", new_person) if name)
    new_old_names = new_names & old_names
    unchanged_names.update(new_old_names)


if __name__ == "__main__":
    main()
