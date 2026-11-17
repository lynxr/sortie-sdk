# The universal call from Python: the generated wrappers of the DCS API catalog over the engine
# client. Compiled by python/sortie-gen/tests/test_sdk_snippets.py (it needs a running engine,
# so it is not run there).

from sortie_gen.dcs_api import Dcs
from sortie_gen.engine import EngineClient
from sortie_gen.errors import CommandFailed


# snippet:start gen-dcs-client
def resupply(engine: str, token: str) -> None:
    with EngineClient(engine, token=token) as client:   # токен logic или admin
        dcs = Dcs(client)
        print(dcs.land().surface_type((-281_000.0, 647_000.0)))   # (север, восток) -> "LAND"
        units = dcs.group("SAM").units()                           # [DcsObject, ...]
        print([(u.name, u.eid) for u in units])
        try:
            dcs.warehouse("Kutaisi").add_item("weapons.missiles.AIM_120C", 8, key="resupply-1")
        except CommandFailed as e:
            print(e.code, e.capability)   # REJECTED call:mission:Warehouse.addItem
# snippet:end gen-dcs-client
