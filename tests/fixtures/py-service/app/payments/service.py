from dataclasses import dataclass

from .gateway import GatewayError, HttpGateway

MAX_RETRY_COUNT = 3


@dataclass
class ChargeResult:
    transaction_id: str
    attempts: int


class PaymentService:
    def __init__(self, gateway: HttpGateway) -> None:
        self.gateway = gateway

    def charge_with_retry(self, amount_cents: int, token: str) -> ChargeResult:
        if amount_cents <= 0:
            raise ValueError("amount must be positive")
        last_error: Exception | None = None
        for attempt in range(1, MAX_RETRY_COUNT + 1):
            try:
                return ChargeResult(self.gateway.charge(amount_cents, token), attempt)
            except GatewayError as error:
                last_error = error
        raise RuntimeError("payment failed after retries") from last_error
