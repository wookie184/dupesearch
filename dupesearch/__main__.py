from . import cli_utils
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
    type=click.Path(exists=True, file_okay=False, readable=False, resolve_path=True),
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
    "--file-path",
    "-p",
    "file_path",
    type=click.File("w", "utf8"),
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
@click.option(  # TODO: implement this
    "--file-formats",
    "-f",
    "file_formats",
    type=str,
    help=(
        "File formats to search for, defaults to all. Supply as comma separated list."
        "`photo` and `video` can be supplied to include common extensions respectively."
    ),
)
@click.option("--quiet", "-q", flag_value=False, help="Suppress all command output")
@click.help_option("--help", "-h")
def dupesearch_cli(
    interactive, save_to_file, file_path, search_path, quiet, delete_duplicates, file_formats
):
    """Find and remove duplicate files quickly."""
    if interactive:
        search_path = cli_utils.ask_for_path()

    dupefinder = cli_utils.get_duplicates(search_path, not interactive or quiet)

    if len(dupefinder.duplicates) == 0:
        return

    if interactive:
        cli_utils.process_results(dupefinder)
    else:
        if save_to_file:
            cli_utils.save_to_file(dupefinder.duplicates, file_path, quiet)

        if delete_duplicates:
            cli_utils.delete_files(dupefinder, quiet)


if __name__ == "__main__":
    dupesearch_cli()
