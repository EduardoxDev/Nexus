# API HTTP

| Método | Rota | Descrição |
|---|---|---|
| POST | `/api/auth/login` | Login com e-mail e senha; cria sessão |
| POST | `/api/auth/logout` | Revoga a sessão |
| GET | `/api/auth/oauth/:provider/callback` | Retorno do provedor OAuth |
| GET | `/api/orders` | Lista pedidos do usuário |
| POST | `/api/orders/checkout` | Finaliza o carrinho e cobra o cliente |
| POST | `/api/billing/subscription` | Troca o plano de assinatura |
