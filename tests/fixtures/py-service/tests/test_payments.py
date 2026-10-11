import pytest

from app.payments.gateway import GatewayError
from app.payments.service import PaymentService


class FlakyGateway:
    def __init__(self, failures: int) -> None:
        self.failures = failures

    def charge(self, amount_cents: int, token: str) -> str:
        if self.failures:
            self.failures -= 1
            raise GatewayError("temporário")
        return "tx_42"


def test_charge_retries_until_success():
    result = PaymentService(FlakyGateway(failures=2)).charge_with_retry(1000, "tok")
    assert result.transaction_id == "tx_42"
    assert result.attempts == 3


def test_rejects_non_positive_amount():
    with pytest.raises(ValueError):
        PaymentService(FlakyGateway(0)).charge_with_retry(0, "tok")
