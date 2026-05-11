#!/usr/bin/env python3
"""Ingest markdown meeting transcripts into Open Brain and Chloe-pied tasks.

The script scans a folder of `.md`/`.markdown` transcripts, extracts Scott-owned
action items, ingests each transcript into Open Brain, then creates Chloe-pied
tasks through the `chloe-pied add-task` CLI. It keeps JSONL audit logs so a
transcript is only processed once unless `--force` is used.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

SOURCE = "meeting-transcript"
ROLE = "meeting_transcript"
DEFAULT_AUDIT_DIR = Path(".chloe-pied/meeting-transcript-ingest")

EXTRACTION_SCHEMA: dict[str, Any] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["action_items"],
    "properties": {
        "action_items": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["title", "description", "evidence"],
                "properties": {
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "evidence": {"type": "string"},
                    "task_type": {
                        "type": "string",
                        "enum": ["task", "bug", "feature", "chore"],
                        "default": "task",
                    },
                },
            },
        }
    },
}

EXTRACTION_PROMPT = """\
You extract ONLY Scott Roy-owned action items from a markdown meeting transcript.

Rules:
- Return JSON only, matching the provided schema.
- Include an item only when Scott owns the next action, explicitly or by clear meeting context.
- Exclude actions owned by other speakers.
- Exclude vague discussion, status updates, and already-completed work.
- Convert each action into a concise Chloe-pied task title and an implementation-ready description.
- Include short evidence quoted or paraphrased from the transcript.
"""

SCOTT_ACTION_PATTERNS = [
    re.compile(r"\b(?:i(?:'|’)ll|i will|i need to|i have to|i can|i should|i'm going to|i am going to|let me|i'll have to|i can see|i can check|i can take|i can add)\b", re.I),
    re.compile(r"\b(?:we need to|we should|we can|first step is|fair point to action)\b", re.I),
]
FILLER_RE = re.compile(r"\b(?:um|uh|mhm|yeah|okay|cool|so|like|you know)\b", re.I)
SPEAKER_RE = re.compile(r"^\*\*(?P<speaker>[^*]+):\*\*\s*(?P<text>.*)")
TIMESTAMP_RE = re.compile(r"^#{1,6}\s+\d{2}:\d{2}:\d{2}")


@dataclass
class ActionItem:
    title: str
    description: str
    evidence: str
    task_type: str = "task"


@dataclass
class TranscriptResult:
    path: Path
    digest: str
    actions: list[ActionItem]
    transcript_message_id: str | None
    task_ids: list[str]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path, help="Folder containing markdown transcripts")
    parser.add_argument("--cli", default=os.environ.get("CHLOE_PIED_CLI", "cargo run --quiet --"), help="Chloe-pied CLI command prefix")
    parser.add_argument("--audit-dir", type=Path, default=DEFAULT_AUDIT_DIR, help="Directory for processed/task JSONL audit logs")
    parser.add_argument("--extractor-command", default=os.environ.get("MEETING_TRANSCRIPT_EXTRACTOR_COMMAND"), help="Optional command that reads isolated prompt JSON from stdin and returns schema JSON")
    parser.add_argument("--max-tasks-per-transcript", type=int, default=8)
    parser.add_argument("--limit", type=int, help="Maximum number of transcripts to process")
    parser.add_argument("--dry-run", action="store_true", help="Run extraction and CLI dry-run without DB writes or processed-file marking")
    parser.add_argument("--force", action="store_true", help="Process transcripts even if already marked processed")
    parser.add_argument("--no-open-brain", action="store_true", help="Skip Open Brain ingestion")
    parser.add_argument("--verbose", action="store_true")
    return parser.parse_args()


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def scan_transcripts(folder: Path) -> list[Path]:
    if not folder.exists() or not folder.is_dir():
        raise SystemExit(f"Transcript folder not found: {folder}")
    paths = [p for p in folder.rglob("*") if p.is_file() and p.suffix.lower() in {".md", ".markdown"}]
    return sorted(paths, key=lambda path: str(path).lower())


def read_processed(audit_dir: Path) -> set[str]:
    processed_path = audit_dir / "processed_transcripts.jsonl"
    if not processed_path.exists():
        return set()
    digests: set[str] = set()
    with processed_path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            digest = record.get("sha256")
            if digest:
                digests.add(digest)
    return digests


def append_jsonl(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")


def clean_text(text: str) -> str:
    text = re.sub(r"\s+", " ", text).strip(" -–—:\t\n")
    return FILLER_RE.sub("", text).strip()


def task_title_from_text(text: str) -> str:
    text = clean_text(text)
    text = re.sub(r"^(?:i(?:'|’)ll|i will|i need to|i have to|i can|i should|i(?:'|’)m going to|i am going to|let me|we need to|we should|we can)\s+", "", text, flags=re.I)
    text = text[:110].rstrip(" .,;:")
    return text[:1].upper() + text[1:] if text else "Follow up on meeting action item"


def split_sentences(text: str) -> list[str]:
    return [part.strip() for part in re.split(r"(?<=[.!?])\s+", text) if part.strip()]


def heuristic_extract_actions(transcript: str, max_items: int) -> list[ActionItem]:
    actions: list[ActionItem] = []
    current_timestamp = ""
    for raw_line in transcript.splitlines():
        line = raw_line.strip().replace("\u00a0", " ")
        if not line:
            continue
        if TIMESTAMP_RE.match(line):
            current_timestamp = line.lstrip("# ").strip()
            continue
        match = SPEAKER_RE.match(line)
        if not match:
            continue
        speaker = match.group("speaker").strip().lower()
        if speaker not in {"scott roy", "scott"}:
            continue
        text = match.group("text").strip()
        for sentence in split_sentences(text):
            if any(pattern.search(sentence) for pattern in SCOTT_ACTION_PATTERNS):
                title = task_title_from_text(sentence)
                evidence = f"{current_timestamp} Scott Roy: {sentence}" if current_timestamp else f"Scott Roy: {sentence}"
                description = textwrap.dedent(f"""\
                Scott-owned action item extracted from meeting transcript.

                Evidence: {evidence}
                """).strip()
                actions.append(ActionItem(title=title, description=description, evidence=evidence))
                if len(actions) >= max_items:
                    return dedupe_actions(actions)
    return dedupe_actions(actions)


def dedupe_actions(actions: Iterable[ActionItem]) -> list[ActionItem]:
    seen: set[str] = set()
    deduped: list[ActionItem] = []
    for action in actions:
        key = re.sub(r"\W+", " ", action.title.lower()).strip()
        if not key or key in seen:
            continue
        seen.add(key)
        deduped.append(action)
    return deduped


def run_external_extractor(command: str, transcript_path: Path, transcript: str, max_items: int) -> list[ActionItem]:
    payload = {
        "prompt": EXTRACTION_PROMPT,
        "schema": EXTRACTION_SCHEMA,
        "transcript_path": str(transcript_path),
        "transcript_markdown": transcript,
    }
    completed = subprocess.run(
        shlex.split(command),
        input=json.dumps(payload, ensure_ascii=False),
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"extractor failed ({completed.returncode}): {completed.stderr.strip()}")
    data = json.loads(completed.stdout)
    return validate_actions(data.get("action_items", []), max_items)


def validate_actions(raw_actions: Any, max_items: int) -> list[ActionItem]:
    if not isinstance(raw_actions, list):
        raise ValueError("extractor output action_items must be an array")
    actions: list[ActionItem] = []
    for raw in raw_actions[:max_items]:
        if not isinstance(raw, dict):
            continue
        title = str(raw.get("title", "")).strip()
        description = str(raw.get("description", "")).strip()
        evidence = str(raw.get("evidence", "")).strip()
        task_type = str(raw.get("task_type", "task")).strip().lower() or "task"
        if title and description and evidence and task_type in {"task", "bug", "feature", "chore"}:
            actions.append(ActionItem(title=title, description=description, evidence=evidence, task_type=task_type))
    return dedupe_actions(actions)


def extract_actions(args: argparse.Namespace, transcript_path: Path, transcript: str) -> list[ActionItem]:
    if args.extractor_command:
        return run_external_extractor(args.extractor_command, transcript_path, transcript, args.max_tasks_per_transcript)
    return heuristic_extract_actions(transcript, args.max_tasks_per_transcript)


def open_brain_connection() -> Any:
    try:
        import psycopg2  # type: ignore
    except ImportError as error:
        raise RuntimeError("psycopg2 is required for Open Brain ingestion") from error

    database_url = os.environ.get("OPEN_BRAIN_DATABASE_URL") or os.environ.get("DATABASE_URL")
    if database_url:
        return psycopg2.connect(database_url)

    required = {
        "host": os.environ.get("OPEN_BRAIN_DB_HOST"),
        "dbname": os.environ.get("OPEN_BRAIN_DB_NAME"),
        "user": os.environ.get("OPEN_BRAIN_DB_USER"),
        "password": os.environ.get("OPEN_BRAIN_DB_PASSWORD"),
        "port": os.environ.get("OPEN_BRAIN_DB_PORT", "5432"),
    }
    env_names = {
        "host": "OPEN_BRAIN_DB_HOST",
        "dbname": "OPEN_BRAIN_DB_NAME",
        "user": "OPEN_BRAIN_DB_USER",
        "password": "OPEN_BRAIN_DB_PASSWORD",
    }
    missing = [env_names[name] for name, value in required.items() if name in env_names and not value]
    if missing:
        raise RuntimeError(f"missing Open Brain DB env vars: {', '.join(missing)}")
    return psycopg2.connect(**required)


def ingest_open_brain(transcript_path: Path, transcript: str, digest: str) -> str:
    session_id = f"meeting-transcripts:{digest[:16]}"
    message_id = f"meeting-transcript:{digest}"
    timestamp = dt.datetime.fromtimestamp(transcript_path.stat().st_mtime, dt.timezone.utc)
    content = f"# Source transcript: {transcript_path}\n\n{transcript}"
    conn = open_brain_connection()
    try:
        with conn:
            with conn.cursor() as cursor:
                cursor.execute(
                    """
                    INSERT INTO sessions (id, machine, project, cwd, started_at, label)
                    VALUES (%s, %s, %s, %s, %s, %s)
                    ON CONFLICT (id) DO NOTHING
                    """,
                    (session_id, os.uname().nodename, "chloe-pied", str(Path.cwd()), timestamp, transcript_path.name),
                )
                cursor.execute(
                    """
                    INSERT INTO messages (id, session_id, role, content, timestamp, source)
                    VALUES (%s, %s, %s, %s, %s, %s)
                    ON CONFLICT (id) DO NOTHING
                    """,
                    (message_id, session_id, ROLE, content, timestamp, SOURCE),
                )
    finally:
        conn.close()
    return message_id


def build_task_description(action: ActionItem, transcript_path: Path, message_id: str | None) -> str:
    parts = [action.description.strip(), "", f"Evidence: {action.evidence}"]
    if message_id:
        parts.extend(["", f"Open Brain message: {message_id}"])
    return "\n".join(parts).strip()


def invoke_chloe_cli(cli_prefix: str, action: ActionItem, transcript_path: Path, message_id: str | None, dry_run: bool) -> str | None:
    command = shlex.split(cli_prefix) + [
        "add-task",
        "--title",
        action.title,
        "--description",
        build_task_description(action, transcript_path, message_id),
        "--task-type",
        action.task_type,
        "--source",
        SOURCE,
        "--transcript-path",
        str(transcript_path),
    ]
    if dry_run:
        command.append("--dry-run")
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    if completed.returncode != 0:
        raise RuntimeError(f"chloe CLI failed ({completed.returncode}): {completed.stderr.strip()}")
    output = completed.stdout.strip()
    json_start = output.find("{")
    if json_start == -1:
        return None
    task = json.loads(output[json_start:])
    return task.get("id")


def process_transcript(args: argparse.Namespace, path: Path, processed: set[str]) -> TranscriptResult | None:
    transcript = path.read_text(encoding="utf-8")
    digest = sha256_text(transcript)
    if digest in processed and not args.force:
        if args.verbose:
            print(f"skip already processed: {path}")
        return None

    actions = extract_actions(args, path, transcript)
    message_id = None if args.dry_run or args.no_open_brain else ingest_open_brain(path, transcript, digest)
    task_ids: list[str] = []

    for action in actions:
        task_id = invoke_chloe_cli(args.cli, action, path, message_id, args.dry_run)
        if task_id:
            task_ids.append(task_id)
        append_jsonl(
            args.audit_dir / "created_tasks.jsonl",
            {
                "timestamp": now_iso(),
                "dry_run": args.dry_run,
                "task_id": task_id,
                "title": action.title,
                "source": SOURCE,
                "source_transcript": str(path),
                "transcript_sha256": digest,
                "open_brain_message_id": message_id,
            },
        )

    if not args.dry_run:
        append_jsonl(
            args.audit_dir / "processed_transcripts.jsonl",
            {
                "timestamp": now_iso(),
                "source": SOURCE,
                "source_transcript": str(path),
                "sha256": digest,
                "open_brain_message_id": message_id,
                "tasks_created": len(task_ids),
                "task_ids": task_ids,
            },
        )
        processed.add(digest)

    return TranscriptResult(path=path, digest=digest, actions=actions, transcript_message_id=message_id, task_ids=task_ids)


def main() -> int:
    args = parse_args()
    args.audit_dir.mkdir(parents=True, exist_ok=True)
    processed = read_processed(args.audit_dir)
    paths = scan_transcripts(args.folder)
    if args.limit:
        paths = paths[: args.limit]

    results: list[TranscriptResult] = []
    for path in paths:
        try:
            result = process_transcript(args, path, processed)
        except Exception as error:  # continue processing remaining transcripts
            append_jsonl(args.audit_dir / "errors.jsonl", {"timestamp": now_iso(), "source_transcript": str(path), "error": str(error)})
            print(f"ERROR {path}: {error}", file=sys.stderr)
            continue
        if result:
            results.append(result)
            print(f"{path}: {len(result.actions)} action(s), {len(result.task_ids)} task id(s){' [dry-run]' if args.dry_run else ''}")

    summary = {
        "dry_run": args.dry_run,
        "transcripts_seen": len(paths),
        "transcripts_processed": len(results),
        "tasks_extracted": sum(len(result.actions) for result in results),
        "task_ids": [task_id for result in results for task_id in result.task_ids],
        "audit_dir": str(args.audit_dir),
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
