from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


@dataclass(frozen=True)
class Scene:
    description: str
    end_time: float
    objects: List[str] = field(default_factory=list)


@dataclass(frozen=True)
class TranscriptSegment:
    start_time: float
    end_time: float
    text: str


@dataclass(frozen=True)
class ProcessingResult:
    """Scenes, transcript, and metadata for a completed job.

    ``raw`` contains the full API response dict if you need fields
    not mapped here.
    """

    job_id: str
    status: str
    filename: str
    duration: float
    scenes: List[Scene]
    transcript: List[TranscriptSegment]
    created_at: str
    raw: Dict[str, Any]


@dataclass
class Job:
    """Tracks a video processing job. Poll ``is_terminal`` to know when it's done."""

    id: str
    status: str
    filename: str
    created_at: str
    eta_seconds: Optional[float]
    raw: Dict[str, Any]

    @property
    def is_terminal(self) -> bool:
        return self.status in {"COMPLETED", "COMPLETED_NO_SCENES", "FAILED"}

    @property
    def is_complete(self) -> bool:
        return self.status in {"COMPLETED", "COMPLETED_NO_SCENES"}

    @property
    def is_failed(self) -> bool:
        return self.status == "FAILED"


@dataclass(frozen=True)
class Quota:
    plan: str
    included_hours: float
    credits_balance_hours: float
    reset_date: Optional[str]


@dataclass
class JobPage:
    jobs: List[Job]
    next_cursor: Optional[str]

    @property
    def has_more(self) -> bool:
        return self.next_cursor is not None


def _parse_scene(data: Dict[str, Any]) -> Scene:
    return Scene(
        description=str(data.get("description", "")),
        end_time=float(data.get("endTs", 0.0)),
        objects=list(data.get("objects", [])),
    )


def _parse_transcript_segment(data: Dict[str, Any]) -> TranscriptSegment:
    return TranscriptSegment(
        start_time=float(data.get("StartTime", 0.0)),
        end_time=float(data.get("EndTime", 0.0)),
        text=str(data.get("Text", "")),
    )


def _parse_job(data: Dict[str, Any]) -> Job:
    return Job(
        id=str(data.get("jobId", "")),
        status=str(data.get("status", "")),
        filename=str(data.get("originalFilename", "")),
        created_at=str(data.get("createdAt", "")),
        eta_seconds=data.get("estimatedCompletionTimeSeconds"),
        raw=data,
    )


def _parse_result(data: Dict[str, Any]) -> ProcessingResult:
    processed = data.get("processedData") or {}
    scenes_raw = processed.get("scenes") or []
    transcript_raw = processed.get("transcript") or []

    return ProcessingResult(
        job_id=str(data.get("jobId", "")),
        status=str(data.get("status", "")),
        filename=str(data.get("originalFilename", "")),
        duration=float(processed.get("length", 0.0)),
        scenes=[_parse_scene(s) for s in scenes_raw],
        transcript=[_parse_transcript_segment(t) for t in transcript_raw],
        created_at=str(data.get("createdAt", "")),
        raw=data,
    )


def _parse_quota(data: Dict[str, Any]) -> Quota:
    return Quota(
        plan=str(data.get("plan", "")),
        included_hours=float(data.get("includedHours", 0.0)),
        credits_balance_hours=float(data.get("creditsBalanceHours", 0.0)),
        reset_date=data.get("resetDate"),
    )
