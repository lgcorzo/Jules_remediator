from abc import ABC, abstractmethod
from typing import List, Optional
from src.domain.models import ClusterError, FixProposal, Severity, RiskScore

class RemediationStrategy(ABC):
    @abstractmethod
    def evaluate(self, error: ClusterError) -> bool:
        pass

class HighSeverityTrigger(RemediationStrategy):
    def evaluate(self, error: ClusterError) -> bool:
        return error.severity in [Severity.HIGH, Severity.CRITICAL]

class RemediationService:
    def __init__(self, strategies: List[RemediationStrategy]):
        self.strategies = strategies

    def should_trigger_remediation(self, error: ClusterError) -> bool:
        return any(strategy.evaluate(error) for strategy in self.strategies)

    def select_best_proposal(self, proposals: List[FixProposal]) -> Optional[FixProposal]:
        if not proposals:
            return None
        # Example logic: Filter out high-risk proposals and pick the highest confidence one
        safe_proposals = [p for p in proposals if p.risk_score != RiskScore.HIGH]
        if not safe_proposals:
            return None
        return max(safe_proposals, key=lambda p: p.confidence)
