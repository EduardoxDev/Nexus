import httpx


class GatewayError(Exception):
    """Falha ao falar com o gateway de pagamentos."""


class HttpGateway:
    def __init__(self, base_url: str, api_key_env: str = "PAYMENTS_API_KEY") -> None:
        self.base_url = base_url
        self.api_key_env = api_key_env

    def charge(self, amount_cents: int, token: str) -> str:
        response = httpx.post(
            f"{self.base_url}/charges",
            json={"amount": amount_cents, "token": token},
            timeout=10,
        )
        if response.status_code != 201:
            raise GatewayError(response.text)
        return response.json()["id"]
