export interface Strings {
  subtitle: string;
  description: string;
  changeKey: string;
  cancel: string;
  hint: string;
  keyError: string;
  capsLock: string;
  credit: string;
}

export interface Locale {
  code: string;
  label: string;
  strings: Strings;
}

export const LOCALES: Locale[] = [
  {
    code: "en",
    label: "English",
    strings: {
      subtitle: "Switch desktops",
      description:
        "Hold the key and scroll the mouse wheel to move between your virtual desktops.",
      changeKey: "Change key",
      cancel: "Cancel",
      hint: "Press the new key. Esc to cancel.",
      keyError: "That key can't be used. Try Win, Alt, Shift, Caps Lock or F1–F12.",
      capsLock: "Caps Lock",
      credit: "Made with ♥ by",
    },
  },
  {
    code: "es",
    label: "Español",
    strings: {
      subtitle: "Escritorios virtuales",
      description:
        "Mantén la tecla pulsada y gira la rueda del mouse para moverte entre tus escritorios virtuales.",
      changeKey: "Cambiar tecla",
      cancel: "Cancelar",
      hint: "Pulsa la nueva tecla. Esc para cancelar.",
      keyError: "Esa tecla no se puede usar. Prueba con Win, Alt, Shift, Bloq Mayús o F1–F12.",
      capsLock: "Bloq Mayús",
      credit: "Con ♥ por",
    },
  },
  {
    code: "pt",
    label: "Português",
    strings: {
      subtitle: "Alternar áreas de trabalho",
      description:
        "Segure a tecla e role a roda do mouse para alternar entre suas áreas de trabalho virtuais.",
      changeKey: "Mudar tecla",
      cancel: "Cancelar",
      hint: "Pressione a nova tecla. Esc para cancelar.",
      keyError: "Essa tecla não pode ser usada. Tente Win, Alt, Shift, Caps Lock ou F1–F12.",
      capsLock: "Caps Lock",
      credit: "Feito com ♥ por",
    },
  },
  {
    code: "fr",
    label: "Français",
    strings: {
      subtitle: "Changer de bureau",
      description:
        "Maintenez la touche enfoncée et faites défiler la molette pour naviguer entre vos bureaux virtuels.",
      changeKey: "Changer la touche",
      cancel: "Annuler",
      hint: "Appuyez sur la nouvelle touche. Échap pour annuler.",
      keyError:
        "Cette touche ne peut pas être utilisée. Essayez Win, Alt, Maj, Verrou Maj ou F1–F12.",
      capsLock: "Verrou Maj",
      credit: "Fait avec ♥ par",
    },
  },
  {
    code: "de",
    label: "Deutsch",
    strings: {
      subtitle: "Desktop wechseln",
      description:
        "Halte die Taste gedrückt und scrolle mit dem Mausrad, um zwischen virtuellen Desktops zu wechseln.",
      changeKey: "Taste ändern",
      cancel: "Abbrechen",
      hint: "Drücke die neue Taste. Esc zum Abbrechen.",
      keyError:
        "Diese Taste kann nicht verwendet werden. Versuche Win, Alt, Umschalt, Festschalten oder F1–F12.",
      capsLock: "Festschalten",
      credit: "Mit ♥ erstellt von",
    },
  },
  {
    code: "it",
    label: "Italiano",
    strings: {
      subtitle: "Cambia desktop",
      description:
        "Tieni premuto il tasto e scorri la rotellina del mouse per spostarti tra i desktop virtuali.",
      changeKey: "Cambia tasto",
      cancel: "Annulla",
      hint: "Premi il nuovo tasto. Esc per annullare.",
      keyError:
        "Quel tasto non può essere usato. Prova Win, Alt, Shift, Bloc Maiusc o F1–F12.",
      capsLock: "Bloc Maiusc",
      credit: "Fatto con ♥ da",
    },
  },
  {
    code: "nl",
    label: "Nederlands",
    strings: {
      subtitle: "Bureaublad wisselen",
      description:
        "Houd de toets ingedrukt en scroll met het muiswiel om tussen virtuele bureaubladen te wisselen.",
      changeKey: "Toets wijzigen",
      cancel: "Annuleren",
      hint: "Druk op de nieuwe toets. Esc om te annuleren.",
      keyError:
        "Die toets kan niet worden gebruikt. Probeer Win, Alt, Shift, Caps Lock of F1–F12.",
      capsLock: "Caps Lock",
      credit: "Gemaakt met ♥ door",
    },
  },
  {
    code: "pl",
    label: "Polski",
    strings: {
      subtitle: "Zmiana pulpitu",
      description:
        "Przytrzymaj klawisz i przewiń kółkiem myszy, aby przełączać wirtualne pulpity.",
      changeKey: "Zmień klawisz",
      cancel: "Anuluj",
      hint: "Naciśnij nowy klawisz. Esc, aby anulować.",
      keyError:
        "Tego klawisza nie można użyć. Spróbuj Win, Alt, Shift, Caps Lock lub F1–F12.",
      capsLock: "Caps Lock",
      credit: "Stworzone z ♥ przez",
    },
  },
  {
    code: "ru",
    label: "Русский",
    strings: {
      subtitle: "Переключение рабочих столов",
      description:
        "Удерживайте клавишу и крутите колесо мыши, чтобы переключаться между рабочими столами.",
      changeKey: "Сменить клавишу",
      cancel: "Отмена",
      hint: "Нажмите новую клавишу. Esc — отмена.",
      keyError:
        "Эту клавишу нельзя использовать. Попробуйте Win, Alt, Shift, Caps Lock или F1–F12.",
      capsLock: "Caps Lock",
      credit: "Сделано с ♥, автор",
    },
  },
  {
    code: "ja",
    label: "日本語",
    strings: {
      subtitle: "デスクトップ切り替え",
      description:
        "キーを押したままマウスのホイールを回すと、仮想デスクトップを切り替えられます。",
      changeKey: "キーを変更",
      cancel: "キャンセル",
      hint: "新しいキーを押してください。Esc で中止。",
      keyError:
        "そのキーは使用できません。Win、Alt、Shift、Caps Lock または F1–F12 をお試しください。",
      capsLock: "Caps Lock",
      credit: "♥ を込めて",
    },
  },
  {
    code: "zh",
    label: "中文（简体）",
    strings: {
      subtitle: "切换桌面",
      description: "按住按键并滚动鼠标滚轮，即可在虚拟桌面之间切换。",
      changeKey: "更改按键",
      cancel: "取消",
      hint: "请按新按键。按 Esc 取消。",
      keyError: "该按键不可用。请尝试 Win、Alt、Shift、Caps Lock 或 F1–F12。",
      capsLock: "大写锁定",
      credit: "用 ♥ 制作",
    },
  },
  {
    code: "ko",
    label: "한국어",
    strings: {
      subtitle: "데스크톱 전환",
      description: "키를 누른 채 마우스 휠을 굴리면 가상 데스크톱 간에 이동합니다.",
      changeKey: "키 변경",
      cancel: "취소",
      hint: "새 키를 누르세요. Esc로 취소.",
      keyError: "이 키는 사용할 수 없습니다. Win, Alt, Shift, Caps Lock 또는 F1–F12를 사용해 보세요.",
      capsLock: "Caps Lock",
      credit: "♥로 만든",
    },
  },
];

export function resolveLocale(saved: string | null, browser: string): Locale {
  if (saved) {
    const match = LOCALES.find((l) => l.code === saved);
    if (match) return match;
  }
  const base = browser.toLowerCase().split("-")[0];
  return LOCALES.find((l) => l.code === base) ?? LOCALES[0];
}
