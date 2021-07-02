import threading
import time
from pathlib import Path
import mimetypes

from rich.progress import BarColumn, Progress, TimeRemainingColumn
from rich.prompt import Prompt
from rich.console import Console
import json

from . import dupesearch


console = Console()

VIDEO_FORMATS = [
    ext.lstrip(".") for ext, mime in mimetypes.types_map.items() if mime.startswith("video")
]
IMAGE_FORMATS = [
    ext.lstrip(".") for ext, mime in mimetypes.types_map.items() if mime.startswith("image")
]
AUDIO_FORMATS = [
    ext.lstrip(".") for ext, mime in mimetypes.types_map.items() if mime.startswith("audio")
]
TEXT_FORMATS = [
    ext.lstrip(".") for ext, mime in mimetypes.types_map.items() if mime.startswith("text")
]


def get_progress_bar():
    progress_bar = Progress(
        "[progress.description]{task.description}",
        BarColumn(),
        "[progress.percentage]{task.percentage:>3.0f}%",
        TimeRemainingColumn(),
        "{task.completed} of {task.total} processed",
        auto_refresh=False,
    )
    return progress_bar


def ask_for_path():
    option = Prompt.ask(
        "Enter the folder path to search in (leave blank to use current directory)",
        default=Path.cwd(),
    )
    path = str(Path(option).absolute())
    return path


def get_duplicates(search_path, file_formats, quiet=False):
    dupefinder = dupesearch.DuplicateFinder(search_path, file_formats)

    thread = threading.Thread(target=dupefinder.find_duplicates)
    thread.start()
    if not quiet:
        console.print(f"Searching for duplicate photos at path: {search_path}")
        display_progress_bar(dupefinder)
    thread.join()

    if not quiet:
        count = len(dupefinder.duplicates)
        if count == 0:
            console.print("No duplicates found!")
        else:
            console.print(f"{count} duplicate{'' if count == 1 else 's'} found!")
    return dupefinder


def display_progress_bar(dupefinder):
    with get_progress_bar() as progress:
        finding_files = progress.add_task("Finding Files...")
        while not dupefinder.has_found_files:
            value = dupefinder.file_count
            progress.update(finding_files, completed=value, total=value)
            progress.refresh()
            time.sleep(0.1)
        value = dupefinder.file_count
        progress.update(finding_files, completed=value, total=value)
        progress.stop_task(finding_files)

        processing_files = progress.add_task("Processing Files...", total=dupefinder.file_count)
        while not dupefinder.has_processed_files:
            progress.update(processing_files, completed=dupefinder.processed_count)
            progress.refresh()
            time.sleep(0.1)
        progress.update(processing_files, completed=dupefinder.processed_count)
        progress.stop_task(processing_files)

        finding_dupes = progress.add_task("Getting Duplicates...", start=True)
        while not dupefinder.has_finished:
            progress.refresh()
            time.sleep(0.1)
        dupes_found = len(dupefinder.duplicates)
        progress.update(finding_dupes, total=dupes_found, completed=dupes_found)
        progress.stop_task(finding_dupes)


def save_to_file(dupes, file_path, quiet=False):
    with open(file_path, "w") as f:
        json.dump(dupes, f, indent=4)

    if not quiet:
        console.print(f"Output saved to file: {file_path}")


def ask_for_file_formats():
    choices = Prompt.ask(
        "Would you like to filter by file extension? If not, press enter, otherwise enter "
        "a list of comma separated extensions. You can enter photo, video, text, or audio "
        "to select common extensions in those groups respectively"
    )
    return choices


def ask_for_file_name():
    while True:
        location = Prompt.ask("Enter the path and/or file name to save the file to")
        path = Path(location).absolute()
        print(path)
        if not path.is_dir():
            path = path if path.suffix.endswith("json") else path.with_suffix(path.suffix + ".json")
        else:
            path = path / "duplicates.json"

        if path.exists():
            console.print(
                f"The file `{path}` already exists. Please delete this file, or enter a new path"
            )
            continue

        if not path.parent.exists():
            console.print(
                f"The directory {path.parent} for that file does not exist. "
                "Please enter a new path"
            )
            continue

        return path


def process_results(dupefinder):
    option = Prompt.ask(
        "What would you like to do next?",
        choices=["delete", "save", "exit"],
        default="delete",
    )
    if option == "save":
        path = ask_for_file_name()
        save_to_file(dupefinder.duplicates, path)
    elif option == "delete":
        delete_files(dupefinder)


def delete_files(dupefinder, quiet=False):
    thread = threading.Thread(target=dupefinder.delete_duplicates)
    thread.start()
    if not quiet:
        with get_progress_bar() as progress:
            deleting = progress.add_task("Deleting Duplicates...", total=len(dupefinder.duplicates))
            while thread.is_alive():
                progress.update(deleting, completed=dupefinder.deleted_count)
                time.sleep(0.1)
            progress.update(deleting, completed=dupefinder.deleted_count)

    thread.join()

    if not quiet:
        console.print("Duplicates have been deleted!")


def parse_file_formats(raw_formats):
    if not raw_formats or raw_formats.strip() == "":
        return None

    formats = raw_formats.split(",")
    cleaned = []
    for format in formats:
        format = format.strip().lower()
        if format in ("photo", "photos", "image", "images"):
            cleaned.extend(IMAGE_FORMATS)
        elif format in ("video", "videos"):
            cleaned.extend(VIDEO_FORMATS)
        elif format in ("text",):
            cleaned.extend(TEXT_FORMATS)
        elif format in ("audio", "sound"):
            cleaned.extend(AUDIO_FORMATS)
        elif format == "all":
            cleaned = None
            break
        elif format.startswith("."):
            cleaned.append(format.lstrip("."))
        else:
            raise ValueError(f"{format} is not a valid option")
    return cleaned
