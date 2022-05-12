"""Console APIs."""

from __future__ import annotations

import enum

import rich.prompt
import rich.text


@enum.unique
class Command(enum.Enum):
    """User command."""

    ARCHIVE = "archive"
    DELETE = "delete"
    FAVORITE = "favorite"
    NEXT = "next"
    QUIT = "quit"

    def __str__(self) -> str:
        """Returns user-facing command string."""
        return self.value

    @staticmethod
    def parse(text: str) -> Command | None:
        """Returns matching command, supports prefix matching, or None if not found."""
        if not text:
            # explicitly check empty string to avoid matching first command
            return None
        for cmd in Command:
            if cmd.value.startswith(text):
                return cmd
        return None


class CommandPrompt(rich.prompt.PromptBase[Command]):
    """Prompt for user command."""

    response_type = Command
    validate_error_message = "[prompt.invalid]Please enter a valid command"
    choices: list[str] = [str(cmd) for cmd in list(Command)]

    def process_response(self, value: str) -> Command:
        """Converts choice to a Command."""
        value = value.strip().lower()
        if (cmd := Command.parse(value)) is not None:
            return cmd
        raise rich.prompt.InvalidResponse(self.validate_error_message)
