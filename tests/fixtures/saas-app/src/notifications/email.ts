import nodemailer from "nodemailer";

import { loadConfig } from "../config";
import { log } from "../utils/logger";
import { orderConfirmationTemplate } from "./templates";

const transport = nodemailer.createTransport({ host: loadConfig().smtpHost, port: 25 });

/** Envia o e-mail de confirmação do pedido. */
export async function sendOrderConfirmation(userId: string, orderId: string, total: number): Promise<void> {
  const { subject, body } = orderConfirmationTemplate(orderId, total);
  await transport.sendMail({ to: `${userId}@users.acme.test`, subject, text: body });
  log("info", "email enviado", { orderId });
}
