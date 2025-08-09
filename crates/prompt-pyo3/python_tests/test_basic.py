"""
Basic tests for the prompt Python bindings.
"""


def test_import() -> None:
    """Test that the package can be imported."""
    import prompt

    assert hasattr(prompt, "__version__")


def test_version() -> None:
    """Test that version is accessible."""
    import prompt

    version = prompt.__version__
    assert isinstance(version, str)
    assert len(version) > 0
