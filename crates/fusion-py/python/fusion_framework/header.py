"""HTTP header name constants and helpers (from fusion-core).

Constants are header *names* (and common media types)::

    from fusion_framework import header

    header.CONTENT_TYPE          # "Content-Type"
    header.APPLICATION_JSON      # "application/json"

Parameterized helpers return a dict you can pass as ``headers=``::

    return self.response(
        data,
        status=status.HTTP_SUCCESS,
        headers=header.download("report.pdf", header.APPLICATION_PDF),
    )

    # or merge:
    headers={
        **header.content_type(header.TEXT_CSV),
        **header.attachment("export.csv"),
    }
"""

from fusion_framework._fusion.header import *  # noqa: F403
from fusion_framework._fusion import header as _header

__all__ = list(getattr(_header, "__all__", []))
