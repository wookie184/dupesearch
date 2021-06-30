import threading
import time
from pathlib import Path

from rich.progress import BarColumn, Progress, TimeRemainingColumn
from rich.prompt import Prompt
from rich.console import Console
import json

from . import dupesearch


console = Console()


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
    console.print(f"Searching for duplicate photos at path: {path}")
    return path


def get_duplicates(search_path, quiet=False):
    dupefinder = dupesearch.DuplicateFinder(search_path)

    thread = threading.Thread(target=dupefinder.find_duplicates)
    thread.start()
    if not quiet:
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

        processing_files = progress.add_task(
            "Processing Files...", total=dupefinder.file_count
        )
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
        json.dump(dupes, f)

    if not quiet:
        console.print(f"Output saved to path: {file_path}")


def process_results(dupefinder):
    option = Prompt.ask(
        "What would you like to do next?",
        choices=["delete", "save", "exit"],
        default="delete",
    )
    if option == "save":
        save_to_file("JSON", dupefinder.duplicates)
    elif option == "delete":
        delete_files(dupefinder)


def delete_files(dupefinder, quiet=False):
    thread = threading.Thread(target=dupefinder.delete_duplicates)
    thread.start()
    if not quiet:
        with get_progress_bar() as progress:
            deleting = progress.add_task(
                "Deleting Duplicates...", total=len(dupefinder.duplicates)
            )
            while thread.is_alive():
                progress.update(deleting, completed=dupefinder.deleted_count)
                time.sleep(0.1)
            progress.update(deleting, completed=dupefinder.deleted_count)

    thread.join()

    if not quiet:
        console.print("Duplicates have been deleted!")
