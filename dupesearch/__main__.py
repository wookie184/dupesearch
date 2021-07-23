from . import cli
from pathlib import Path
import click


@click.command(no_args_is_help=True, context_settings={"max_content_width": 110})
@click.option(
    "--interactive",
    "-i",
    "interactive",
    flag_value=True,
    help="Choose options interactively. If selected, other parameters are ignored.",
)
@click.option(
    "--search-path",
    "-p",
    "search_path",
    default=Path.cwd(),
    type=click.Path(exists=True, file_okay=False, writable=True, resolve_path=True),
    help="Choose search path. Defaults to current directory.",
)
@click.option(
    "--save-to-file",
    "-s",
    "save_to_file",
    flag_value=True,
    help="Save list of duplicate groups found to a file.",
)
@click.option(
    "--save-path",
    "-sp",
    "save_path",
    type=click.Path(exists=False, dir_okay=False, writable=True, resolve_path=True),
    default=Path.cwd() / "duplicates.json",
    show_default=True,
    help="Choose file to save output to, see GitHub readme for details.",
)
@click.option(
    "--delete",
    "-d",
    "delete_duplicates",
    flag_value=True,
    help="Delete duplicates. Will keep the file with the shortest path.",
)
@click.option(
    "--file-formats",
    "-f",
    "file_formats",
    type=str,
    help=(
        "File formats to search for, defaults to all. Supply as comma separated list."
        "`photo` and `video` can be supplied to include common extensions respectively."
    ),
)
@click.option("--quiet", "-q", flag_value=True, help="Suppress all command output")
@click.help_option("--help", "-h")
def dupesearch_cli(
    interactive,
    save_to_file,
    save_path,
    search_path,
    quiet,
    delete_duplicates,
    file_formats,
):
    """Find and remove duplicate files quickly."""
    if interactive:
        search_path = cli.ask_for_path()
        file_formats = cli.ask_for_file_formats()

    formats = cli.parse_file_formats(file_formats)
    dupefinder = cli.get_duplicates(search_path, formats, not interactive and quiet)

    if len(dupefinder.duplicates) == 0:
        return

    if interactive:
        cli.process_results(dupefinder)
    else:
        if save_to_file:
            cli.save_to_file(dupefinder.duplicates, save_path, quiet)

        if delete_duplicates:
            cli.delete_files(dupefinder, quiet)


if __name__ == "__main__":
    dupesearch_cli()
