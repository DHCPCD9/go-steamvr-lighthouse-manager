//! English, Russian, and Simplified Chinese UI strings.
//! Unknown keys fall back to the key itself.

pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ru", "Русский"),
    ("zh_cn", "中文"),
];

type Row = (&'static str, &'static str, &'static str, &'static str);

const TABLE: &[Row] = &[
    ("Devices", "Devices", "Устройства", "设备列表"),
    ("Channel", "Channel", "Канал", "信道"),
    ("Base station channel to operate", "Base station channel to operate", "Канал базовой станции", "基站目标信道"),
    ("Settings", "Settings", "Настройки", "设置"),
    ("Active", "Active", "Активен", "活跃"),
    ("Inactive", "Inactive", "Неактивен", "不活跃"),
    ("Waiting", "Waiting", "Ожидание", "等待中"),
    ("Checking", "Checking", "Проверка", "检查中"),
    ("Server connection failed.", "Server connection failed.", "Соединение с сервером потеряно.", "服务器连接失败。"),
    ("New Group", "New Group", "Новая группа", "新群组"),
    ("No base stations seem to be found, maybe other programs are connected to them?", "No base stations seem to be found, maybe other programs are connected to them?", "Базовых станций не найдено, возможно другие програмы/устройства подключены к им?", "没有发现基站，请检查是否有其他设备连接到了它们。"),
    ("Nickname", "Nickname", "Имя", "昵称"),
    ("Base station nickname to display", "Base station nickname to display", "Имя базовой станции для отображения", "基站显示昵称"),
    ("Manage Power (Wake up)", "Manage Power (Wake up)", "Управление питанием (Включение)", "管理电源（唤醒）"),
    ("Awake this base station on SteamVR Launch", "Awake this base station on SteamVR Launch", "Включать эту базовую станцию при запуске SteamVR", "SteamVR启动时唤醒该基站"),
    ("Manage Power (Power off)", "Manage Power (Power off)", "Управление питанием (Выключение)", "管理电源（关闭）"),
    ("Power off this base station on SteamVR Exit", "Power off this base station on SteamVR Exit", "Погружать эту базовую станцию в сон при выходе из SteamVR", "SteamVR退出时关闭该基站"),
    ("Forget this base station", "Forget this base station", "Забыть эту базовую станцию", "忘记该基站"),
    ("Manage Base Station Power", "Manage Base Station Power", "Управление питанием базовых станций", "管理基站电源状态"),
    ("Manage Power based on SteamVR launched or not", "Manage Power based on SteamVR launched or not", "Включать/Выключать базовые станции в зависимости от SteamVR", "是否跟随SteamVR启动"),
    ("Allow tray", "Allow tray", "Разрешить трей", "允许托盘图标"),
    ("Run the application in the tray when SteamVR is active or when the main window is closed.", "Run the application in the tray when SteamVR is active or when the main window is closed.", "Запускать приложение в трее когда SteamVR активен, или закрыто главное.", "当SteamVR活跃或主窗口关闭时切换至在托盘中运行"),
    ("Tray is unavailable on this platform or could not be initialized.", "Tray is unavailable on this platform or could not be initialized.", "Трей недоступен на этой платформе или не удалось его инициализировать.", "当前平台不支持托盘或托盘初始化失败。"),
    ("Language", "Language", "Язык", "语言"),
    ("Language of the interface", "Language of the interface", "Язык интерфейса", "界面语言"),
    ("Updates", "Updates", "Обновления", "更新"),
    ("Control application updates", "Control application updates", "Управление обновлениями приложения", "控制应用更新"),
    ("Made By", "Made by", "Автор", "由"),
    ("Design By", "Design by", "Дизайн от", "设计"),
    ("Source code", "Source code", "Исходный код", "源码"),
    ("SteamVR not found or this platform is not supported", "SteamVR not found or this platform is not supported", "SteamVR не найден или эта платформа не поддерживаеться", "未发现SteamVR或不支持当前平台"),
    ("Check for updates", "Check for updates", "Проверка на обновления", "检查更新"),
    ("Check for a new version", "Check for a new version", "Проверить, есть ли новые версии этой программы", "检查是否有新版本"),
    ("Software update", "Software update", "Обновление приложения", "软件更新"),
    ("Changing the branch checks for updates; installation is manual.", "Changing the branch checks for updates; installation is manual.", "Смена ветки запускает проверку; установка выполняется вручную.", "切换分支会检查更新；需手动安装。"),
    ("Self-updating is supported on Windows x64 only.", "Self-updating is supported on Windows x64 only.", "Самообновление поддерживается только в Windows x64.", "仅 Windows x64 支持自动更新。"),
    ("Checking for updates…", "Checking for updates…", "Проверка обновлений…", "正在检查更新…"),
    ("No native Rust release is published for this branch yet.", "No native Rust release is published for this branch yet.", "Для этой ветки ещё нет нативного выпуска Rust.", "此分支尚未发布 Rust 原生版本。"),
    ("Update available:", "Update available:", "Доступно обновление:", "可用更新："),
    ("Downloading update", "Downloading update", "Загрузка обновления", "正在下载更新"),
    ("Verifying download…", "Verifying download…", "Проверка загрузки…", "正在验证下载…"),
    ("Preparing update…", "Preparing update…", "Подготовка обновления…", "正在准备更新…"),
    ("Restarting as the current Windows user…", "Restarting as the current Windows user…", "Перезапуск от текущего пользователя Windows…", "正在以当前 Windows 用户身份重启…"),
    ("Update failed:", "Update failed:", "Ошибка обновления:", "更新失败："),
    ("Update branch", "Update branch", "Ветка обновлений", "更新分支"),
    ("Edit group", "Edit group", "Редактировать группу", "编辑群组"),
    ("Group name", "Group name", "Имя группы", "群组名称"),
    ("Display group name", "Display group name", "Отображаемое имя группы", "群组显示名称"),
    ("Manage Power (Power on)", "Manage Power (Power on)", "Управление питанием (Включение)", "管理电源（开启）"),
    ("Wake up base stations on SteamVR Launch", "Wake up base stations on SteamVR Launch", "Включать базовые станции при запуске SteamVR", "SteamVR启动时唤醒基站"),
    ("Manage Power (Power Off)", "Manage Power (Power off)", "Управление питанием (Выключение)", "管理电源（关闭）"),
    ("Power off base stations on SteamVR Exit", "Power off base stations on SteamVR Exit", "Погружать базовые станции в сон при выходе из SteamVR", "SteamVR退出时关闭基站"),
    ("Delete group", "Delete group", "Удалить группу", "删除群组"),
    ("Channels", "Channels", "Каналы", "信道"),
    ("Check", "Check", "Проверить", "检查"),
    ("Update", "Update", "Обновить", "更新"),
    ("You are on the latest version!", "You are on the latest version!", "Вы находитесь на последней версии!", "已经是最新版本了！"),
    ("Window was hidden in the tray.", "Window was hidden in the tray.", "Окно было скрыто в трей.", "窗口被隐藏在托盘中"),
];

/// Translate `key` for the given language code.
pub fn t(lang: &str, key: &str) -> String {
    for row in TABLE {
        if row.0 == key {
            let s = match lang {
                "ru" => row.2,
                "zh_cn" => row.3,
                _ => row.1,
            };
            return s.to_string();
        }
    }
    key.to_string()
}

pub fn language_title(code: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, title)| *title)
        .unwrap_or("English")
}
