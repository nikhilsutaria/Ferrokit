"""Ferrokit: OCRmyPDF's progress as plain lines on stdout, for the job's progress bar.

Each update prints `FERROKIT-PROGRESS <total> <done> <stage>`; the worker turns the stages (scanning,
OCR, finishing) into one percentage and a time left. Loaded with `ocrmypdf --plugin <this file>`.
"""

import sys

from ocrmypdf import hookimpl


class Lines:
    """A progress bar (OCRmyPDF's ProgressBar protocol) that prints instead of drawing."""

    def __init__(self, *, total=None, desc=None, unit=None, disable=False, **kwargs):
        self.total = total or 0
        self.desc = " ".join(str(desc or "").split())
        self.done = 0

    def __enter__(self):
        self._say()
        return self

    def __exit__(self, *exc):
        return False

    def update(self, n=1, *, completed=None):
        if completed is not None:
            self.done = completed
        else:
            self.done += n
        self._say()

    def _say(self):
        print(f"FERROKIT-PROGRESS {self.total:g} {self.done:g} {self.desc}", file=sys.stdout, flush=True)


@hookimpl
def get_progressbar_class():
    return Lines
