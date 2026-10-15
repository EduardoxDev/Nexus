/**
 * Cargos por botão: um painel com um botão por cargo da configuração; cada clique põe ou tira o
 * cargo de quem clicou. O bot só mexe nos cargos de `management.roles` e recusa cargos com
 * permissões de equipe (mesmo que alguém os coloque na configuração por engano).
 */
import {
  ButtonStyle,
  ComponentType,
  type APIActionRowComponent,
  type APIButtonComponent,
} from "discord.js";

import { brandHeader, logoFile, type BotMessage } from "../format.ts";
import type { RoleOption } from "./config.ts";

export const ROLE_PREFIX = "nexus:role:";

export type Components = APIActionRowComponent<APIButtonComponent>[];
export type PanelMessage = BotMessage & { components: Components };

/** Como o Discord vê um cargo agora. */
export interface RoleInfo {
  exists: boolean;
  /** Integração, `@everyone` ou com permissão de equipe (administrar, moderar, gerenciar…). */
  dangerous: boolean;
  /** Abaixo do cargo mais alto do bot (senão o Discord recusa). */
  editable: boolean;
}

/** Fileiras de até 5 botões. */
export function rows(buttons: APIButtonComponent[]): Components {
  const out: Components = [];
  for (let n = 0; n < buttons.length; n += 5) {
    out.push({ type: ComponentType.ActionRow, components: buttons.slice(n, n + 5) });
  }
  return out;
}

export function rolePanel(roles: RoleOption[]): PanelMessage {
  return {
    embeds: [
      {
        ...brandHeader("Cargos"),
        title: "🎭 Escolha seus cargos",
        description: "Clique para receber um cargo; clique de novo para tirar.",
      },
    ],
    files: [logoFile()],
    components: rows(
      roles.map((r) => ({
        type: ComponentType.Button,
        style: ButtonStyle.Secondary,
        custom_id: `${ROLE_PREFIX}${r.roleId}`,
        label: r.label,
        ...(r.emoji ? { emoji: { name: r.emoji } } : {}),
      })),
    ),
  };
}

/** Problemas dos cargos configurados, para avisar antes de publicar o painel. */
export function rolePanelProblems(roles: RoleOption[], info: (id: string) => RoleInfo): string[] {
  const problems: string[] = [];
  for (const r of roles) {
    const i = info(r.roleId);
    if (!i.exists) problems.push(`"${r.label}": o cargo não existe mais`);
    else if (i.dangerous) problems.push(`"${r.label}": cargo com permissões de equipe (recusado)`);
    else if (!i.editable) problems.push(`"${r.label}": o cargo está acima do cargo do bot`);
  }
  return problems;
}

export interface RoleButtonPorts {
  memberRoleIds: string[];
  info(roleId: string): RoleInfo;
  add(roleId: string): Promise<void>;
  remove(roleId: string): Promise<void>;
}

/** Trata o clique; devolve o texto da resposta (só quem clicou vê). */
export async function toggleRole(
  customId: string,
  roles: RoleOption[],
  p: RoleButtonPorts,
): Promise<string> {
  const roleId = customId.slice(ROLE_PREFIX.length);
  const option = roles.find((r) => r.roleId === roleId);
  if (!option) return "Esse cargo não está mais disponível.";
  const info = p.info(roleId);
  if (!info.exists || info.dangerous || !info.editable) {
    return "Não consigo mexer nesse cargo agora. Avise a equipe.";
  }
  try {
    if (p.memberRoleIds.includes(roleId)) {
      await p.remove(roleId);
      return `Cargo **${option.label}** removido.`;
    }
    await p.add(roleId);
    return `Cargo **${option.label}** adicionado.`;
  } catch (e) {
    console.error(`cargo por botão falhou: ${(e as Error).name}`);
    return "Não consegui mudar o cargo. Avise a equipe.";
  }
}
