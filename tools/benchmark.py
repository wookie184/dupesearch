import dupesearch
from pathlib import Path
import string
import random
import tempfile
import time
from typing import NamedTuple, Union, Callable


class TestSet(NamedTuple):
    num_unique: int
    num_total: int
    file_size: Union[int, Callable]  # In bytes


KB = 2 ** 10  # 1 KB in bytes
MB = 2 ** 20  # 1 MB in bytes

TESTSETS = [
    TestSet(num_unique=500, num_total=2000, file_size=KB // 10),
    TestSet(num_unique=1, num_total=2000, file_size=KB),
    TestSet(num_unique=1, num_total=5, file_size=100 * MB),
]


def random_filename():
    return "".join(random.choices(string.ascii_letters + string.digits, k=10))


def prepare_testset(tempdir, testset):
    duplicates_per_group = testset.num_total // testset.num_unique
    assert duplicates_per_group >= 1
    for _ in range(testset.num_unique):
        if callable(testset.file_size):
            file_size = testset.file_size()
        else:
            file_size = testset.file_size

        file_content = random.randbytes(file_size)
        for _ in range(duplicates_per_group):
            file_path = tempdir / random_filename()
            file_path.write_bytes(file_content)


def time_duplicate_finding(tempdir, testset):
    start = time.perf_counter()
    dupefinder = dupesearch.DuplicateFinder(str(tempdir), None)
    dupefinder.find_duplicates()
    end = time.perf_counter()
    if dupefinder.file_count != testset.num_total:
        raise RuntimeError(
            f"Found {dupefinder.file_count} files when there should be {testset.num_total}"
        )
    if len(dupefinder.duplicates) != testset.num_unique:
        raise RuntimeError(
            f"Found {len(dupefinder.duplicates)} duplicate sets when there should be {testset.unique}"
        )
    return end - start


def run_testsets():
    for testset in TESTSETS:
        with tempfile.TemporaryDirectory(suffix="dupesearch-test") as tempdir:
            tempdir = Path(tempdir)
            prepare_testset(tempdir, testset)
            time_taken = time_duplicate_finding(tempdir, testset)
            print(f"{testset} ran in {time_taken:.3g} seconds")


if __name__ == "__main__":
    run_testsets()
