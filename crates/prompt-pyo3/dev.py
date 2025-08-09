#!/usr/bin/env python3
"""
Development helper script for the Python bindings.
Provides common development tasks using uv.
"""

import subprocess
import sys
import os
from pathlib import Path


def run_command(cmd: list[str], description: str) -> bool:
    """Run a command and return success status."""
    print(f"🔧 {description}")
    print(f"   Running: {' '.join(cmd)}")
    
    try:
        result = subprocess.run(cmd, check=True, cwd=Path(__file__).parent)
        print(f"✅ {description} completed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ {description} failed with exit code {e.returncode}")
        return False
    except FileNotFoundError:
        print(f"❌ Command not found: {cmd[0]}")
        return False


def setup_dev():
    """Set up development environment."""
    return run_command(
        ["uv", "sync", "--group", "dev"],
        "Setting up development environment"
    )


def build_dev():
    """Build the extension module for development."""
    return run_command(
        ["uv", "run", "maturin", "develop"],
        "Building extension module for development"
    )


def run_tests():
    """Run the test suite."""
    return run_command(
        ["uv", "run", "pytest", "python_tests/", "-v"],
        "Running test suite"
    )


def run_type_check():
    """Run type checking with mypy."""
    return run_command(
        ["uv", "run", "mypy", "prompt/", "python_tests/"],
        "Running type checking"
    )


def run_lint():
    """Run linting with ruff."""
    success = True
    success &= run_command(
        ["uv", "run", "ruff", "check", "prompt/", "python_tests/"],
        "Running linter (check)"
    )
    success &= run_command(
        ["uv", "run", "ruff", "format", "--check", "prompt/", "python_tests/"],
        "Checking code formatting"
    )
    return success


def fix_lint():
    """Fix linting issues with ruff."""
    success = True
    success &= run_command(
        ["uv", "run", "ruff", "check", "--fix", "prompt/", "python_tests/"],
        "Fixing linting issues"
    )
    success &= run_command(
        ["uv", "run", "ruff", "format", "prompt/", "python_tests/"],
        "Formatting code"
    )
    return success


def build_wheels():
    """Build wheels for distribution."""
    return run_command(
        ["uv", "run", "maturin", "build", "--find-interpreter"],
        "Building wheels for all Python versions"
    )


def clean():
    """Clean build artifacts."""
    import shutil
    
    artifacts = [
        "target/",
        "dist/",
        "*.egg-info/",
        "__pycache__/",
        ".pytest_cache/",
        ".mypy_cache/",
        ".ruff_cache/",
    ]
    
    for pattern in artifacts:
        for path in Path(".").glob(pattern):
            if path.exists():
                if path.is_dir():
                    shutil.rmtree(path)
                    print(f"🗑️  Removed directory: {path}")
                else:
                    path.unlink()
                    print(f"🗑️  Removed file: {path}")
    
    print("✅ Cleanup completed")
    return True


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: python dev.py <command>")
        print("\nAvailable commands:")
        print("  setup     - Set up development environment")
        print("  build     - Build extension module for development")
        print("  test      - Run test suite")
        print("  typecheck - Run type checking")
        print("  lint      - Run linting and format checking")
        print("  fix       - Fix linting issues and format code")
        print("  wheels    - Build distribution wheels")
        print("  clean     - Clean build artifacts")
        print("  all       - Run setup, build, test, typecheck, and lint")
        sys.exit(1)
    
    command = sys.argv[1].lower()
    
    commands = {
        "setup": setup_dev,
        "build": build_dev,
        "test": run_tests,
        "typecheck": run_type_check,
        "lint": run_lint,
        "fix": fix_lint,
        "wheels": build_wheels,
        "clean": clean,
    }
    
    if command == "all":
        success = True
        for cmd_name in ["setup", "build", "test", "typecheck", "lint"]:
            success &= commands[cmd_name]()
            if not success:
                break
        
        if success:
            print("\n🎉 All development tasks completed successfully!")
        else:
            print("\n💥 Some tasks failed. Please check the output above.")
            sys.exit(1)
    
    elif command in commands:
        success = commands[command]()
        if not success:
            sys.exit(1)
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == "__main__":
    main()