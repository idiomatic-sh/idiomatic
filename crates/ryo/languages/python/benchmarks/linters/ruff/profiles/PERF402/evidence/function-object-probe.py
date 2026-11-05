"""Execute complete fixtures unchanged; annotations do not turn functions into instances."""
import hashlib, inspect, json, platform, sys
from pathlib import Path
payload = json.loads(Path(sys.argv[1]).read_text())
records = []
for case in payload:
    namespace = {"item": 1, "xs": [1]}
    error = None
    try:
        exec(compile(case["source"], "case.py", "exec", dont_inherit=True), namespace)
    except Exception as caught:
        error = {"type": type(caught).__name__, "message": str(caught)}
    value = namespace.get("out")
    record = {"rule": case["rule"], "name": case["name"], "source": case["source"],
              "execution_error": error, "out_type": type(value).__name__,
              "out_is_list": isinstance(value, list), "out_is_set": isinstance(value, set),
              "out_is_callable": callable(value)}
    if inspect.isfunction(value):
        record["unchanged_function_call_result_type"] = type(value()).__name__
        assert not record["out_is_list"] and not record["out_is_set"]
        assert error["type"] == ("TypeError" if case["rule"] == "FURB132" else "AttributeError")
    else:
        assert error is None
        assert record["out_is_set" if case["rule"] == "FURB132" else "out_is_list"]
    records.append(record)
print(json.dumps({"implementation": platform.python_implementation(), "version": platform.python_version(),
                  "executable_sha256": hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),
                  "target_configuration": "python-target=3.10", "records": records}, indent=2, sort_keys=True))
