# Тестовые данные sortie-manifest

`edge.manifest.pb` и `edge.manifest.json` проверяют, что `manifest_to_json` пишет ровно то же,
что Python protobuf (`json_format.MessageToJson(m, preserving_proto_field_name=True, indent=2)`),
на неудобных значениях: числа с плавающей точкой (`1e-05`, `1e+16`, `5e-324`, NaN, Infinity),
экранирование строк (кириллица, эмодзи, кавычки, управляющие символы), неизвестные значения enum,
пустые сообщения и map.

Как пересоздать после изменения `edge()` в `tests/json.rs` (из корня репозитория):

```sh
cargo test -p sortie-manifest --test json -- --ignored write_edge_fixture
mkdir -p target/pb   # scratch folder for the generated Python modules (target/ is not in git)
uv run --with grpcio-tools python -m grpc_tools.protoc -Iproto --python_out=target/pb \
    proto/sortie/v1/common.proto proto/sortie/manifest/v1/manifest.proto
uv run --with grpcio-tools python - <<'PY'
import sys; sys.path.insert(0, "target/pb")
from google.protobuf import json_format
from sortie.manifest.v1 import manifest_pb2
d = "crates/sortie-manifest/tests/data/"
m = manifest_pb2.Manifest(); m.ParseFromString(open(d + "edge.manifest.pb", "rb").read())
open(d + "edge.manifest.json", "w").write(
    json_format.MessageToJson(m, preserving_proto_field_name=True, indent=2) + "\n")
PY
```

Эталон получен на CPython 3.13 и protobuf 7.36.
