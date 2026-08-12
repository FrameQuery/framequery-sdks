from __future__ import annotations

import httpx
import pytest
import respx

from framequery import (
    AuthenticationError,
    FrameQuery,
    NotFoundError,
    Scene,
    TranscriptSegment,
)
from framequery._constants import DEFAULT_BASE_URL
from framequery._models import (
    _parse_audio_track_transcript,
    _parse_job,
    _parse_quota,
)

COMPLETED_JOB = {
    "jobId": "job_123",
    "status": "VISION_COMPLETED",
    "originalFilename": "demo.mp4",
    "createdAt": "2026-01-01T00:00:00Z",
    "processedData": {
        "length": 12.5,
        "scenes": [{"description": "intro", "endTs": 3.2, "objects": ["desk", "mug"]}],
        "transcript": [{"StartTime": 0, "EndTime": 2.5, "Text": "hello"}],
    },
}


def test_parse_pending_job() -> None:
    job = _parse_job(
        {
            "jobId": "job_1",
            "status": "PROCESSING",
            "originalFilename": "clip.mp4",
            "estimatedCompletionTimeSeconds": 42,
        }
    )
    assert job.id == "job_1"
    assert job.eta_seconds == 42
    assert not job.is_terminal
    assert not job.is_complete
    assert not job.is_failed
    assert job.result is None


def test_parse_completed_job_with_result() -> None:
    job = _parse_job(COMPLETED_JOB)
    assert job.is_complete
    assert job.is_terminal
    result = job.result
    assert result is not None
    assert result.duration == 12.5
    assert result.scenes == [Scene(description="intro", end_time=3.2, objects=["desk", "mug"])]
    assert result.transcript == [TranscriptSegment(start_time=0.0, end_time=2.5, text="hello")]


def test_parse_failed_job() -> None:
    job = _parse_job({"jobId": "job_2", "status": "VISION_FAILED"})
    assert job.is_failed
    assert job.is_terminal
    assert not job.is_complete


def test_parse_multi_track_fields() -> None:
    job = _parse_job(
        {
            "jobId": "job_3",
            "status": "PROCESSING",
            "audioTrackCount": 2,
            "audioTracksCompleted": 1,
            "audioTrackNames": ["Host", "Guest"],
        }
    )
    assert job.audio_track_count == 2
    assert job.audio_tracks_completed == 1
    assert job.audio_track_names == ["Host", "Guest"]


def test_parse_audio_track_transcript() -> None:
    track = _parse_audio_track_transcript(
        {
            "trackIndex": 1,
            "trackName": "Guest",
            "language": "en",
            "status": "COMPLETED",
            "speakers": ["A"],
            "transcript": [{"StartTime": 1, "EndTime": 2, "Text": "there"}],
        }
    )
    assert track.track_index == 1
    assert track.track_name == "Guest"
    assert track.speakers == ["A"]
    assert track.transcript == [TranscriptSegment(start_time=1.0, end_time=2.0, text="there")]
    assert track.error_message is None


def test_parse_quota() -> None:
    quota = _parse_quota(
        {
            "currentPlan": "pro",
            "includedHours": 10,
            "creditsBalanceHours": 4.5,
            "resetDate": "2026-02-01",
        }
    )
    assert quota.plan == "pro"
    assert quota.included_hours == 10
    assert quota.credits_balance_hours == 4.5
    assert quota.reset_date == "2026-02-01"


def test_client_requires_api_key(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("FRAMEQUERY_API_KEY", raising=False)
    with pytest.raises(ValueError, match="api_key is required"):
        FrameQuery()


@respx.mock
def test_get_job_fetches_and_parses() -> None:
    respx.get(f"{DEFAULT_BASE_URL}/jobs/job_123").mock(
        return_value=httpx.Response(200, json={"data": COMPLETED_JOB})
    )
    with FrameQuery(api_key="test-key") as fq:
        job = fq.get_job("job_123")
    assert job.id == "job_123"
    assert job.is_complete
    assert job.filename == "demo.mp4"


@respx.mock
def test_401_maps_to_authentication_error() -> None:
    respx.get(f"{DEFAULT_BASE_URL}/jobs/job_x").mock(
        return_value=httpx.Response(401, json={"error": "bad key"})
    )
    with FrameQuery(api_key="test-key") as fq:
        with pytest.raises(AuthenticationError, match="bad key"):
            fq.get_job("job_x")


@respx.mock
def test_404_maps_to_not_found_error() -> None:
    respx.get(f"{DEFAULT_BASE_URL}/jobs/job_x").mock(
        return_value=httpx.Response(404, json={"error": "no such job"})
    )
    with FrameQuery(api_key="test-key") as fq:
        with pytest.raises(NotFoundError, match="no such job"):
            fq.get_job("job_x")
