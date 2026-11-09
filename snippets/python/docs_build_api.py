# Building a scenario from Python code instead of `python scenario.py build`.
# Built offline by python/sortie-gen/tests/test_sdk_snippets.py.

from pathlib import Path

from dcs import vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario

sc = Scenario("build_demo", schema=1, theatre=caucasus.Caucasus(), start="2026-06-01T08:00:00Z")
target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3)


# snippet:start gen-build-api
def build_to(out_dir: Path) -> Path:
    b = sc.build()               # в памяти, без движка; build id выводится из содержимого
    for text in b.warnings:      # те же предупреждения, что печатает `build`
        print("warning:", text)
    print(b.build_id, b.digest)  # digest = sha256 manifest.pb, его сверяет bootstrap
    files = b.write(out_dir)     # <build_id>.miz, .manifest.pb, .manifest.json
    return files["miz"]
# snippet:end gen-build-api


if __name__ == "__main__":
    print(build_to(Path("dist")))
