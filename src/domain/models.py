from enum import Enum
from typing import Optional, List
from pydantic import BaseModel, Field
from datetime import datetime

class Severity(str, Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"

class ClusterResource(BaseModel):
    kind: str
    name: str
    namespace: str
    api_version: str

class ClusterError(BaseModel):
    id: str
    timestamp: datetime = Field(default_factory=datetime.utcnow)
    severity: Severity
    resource: ClusterResource
    message: str
    error_code: str
    raw_event: dict = Field(default_factory=dict)

class RiskScore(str, Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"

class FixProposal(BaseModel):
    error_id: str
    proposal_id: str
    code_change: str
    explanation: str
    risk_score: RiskScore
    confidence: float = Field(ge=0.0, le=1.0)
    remediation_command: Optional[str] = None
