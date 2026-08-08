"""HTTP status code constants (from fusion-core).

Example::

    from fusion_framework import status

    print(status.HTTP_SUCCESS)  # 200
    print(status.HTTP_404_NOT_FOUND)
"""

from fusion_framework._fusion.status import *  # noqa: F403
from fusion_framework._fusion import status as _status

__all__ = list(getattr(_status, "__all__", []))
