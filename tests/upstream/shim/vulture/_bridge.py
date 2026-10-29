"""Talk to the condor test bridge (`condor_testapi`): one JSON request per call."""

import json
import os
import subprocess

BRIDGE = os.environ["CONDOR_TESTAPI"]


def call(op, **payload):
    request = json.dumps({"op": op, **payload})
    result = subprocess.run(
        [BRIDGE], input=request, capture_output=True, text=True, check=True
    )
    return json.loads(result.stdout)
