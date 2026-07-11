# 领域术语表 / Domain Glossary

> [← 返回索引](../README.md) · [English glossary](../../en/guides/glossary.md)

本项目涉及大量原神游戏与空荧酒馆地图工具特有的领域术语。以下为中文↔英文↔代码标识符的三语对照表。

## 核心数据域

| 中文 | English | 代码标识符 | 说明 |
| --- | --- | --- | --- |
| 点位 | marker | `marker` 表 / `MarkerVO` | 地图上的一个兴趣点（POI），含坐标、图标、关联物品 |
| 打点 | punctuate | `marker_punctuate` 表 | 众包贡献流程：用户提交点位→编辑审核→晋升为正式 marker |
| 物品 | item | `item` 表 / `ItemVO` | 可收集物（如特产、矿石、书籍等） |
| 图标 | icon | `icon` 表 | marker 的视觉标识图片 |
| 地区 | area | `area` 表 | 游戏地区（蒙德、璃月、稻妻、须弥、枫丹、纳塔、至冬等），树形结构 |
| 路线 | route | `route` 表 | 锄地路线（标记一组按顺序遍历的点位） |
| 标签 | tag | `tag` 表 | 图标标签（已合并入图标表，不再独立） |
| 图标分类 | icon_type / tag_type | `icon_type` / `tag_type` 表 | 图标/标签的分类层级 |
| 点位关联 | marker linkage | `marker_linkage` 表 | 点位之间的连线（如洞穴入口↔出口） |
| 点位-物品关联 | marker-item link | `marker_item_link` 表 | 点位与物品的多对多关系 |

## 游戏专有概念

| 中文 | English | 说明 |
| --- | --- | --- |
| 神瞳 | Oculus | 一种收集品类型（蒙德风神瞳、璃月岩神瞳等），作为图标样式存在 |
| 宝箱 | Chest | 可交互的收集容器，地图上的常见点位类型 |
| 提瓦特 | Teyvat | 原神游戏世界的名称 |
| 内鬼 | Insider / Spy | 拥有测试服/内部数据访问权限的用户角色 |
| 彩蛋 | Easter egg / Surprise | 隐藏的特殊内容标记 |
| 空荧酒馆 | Kongying Tavern | 运营本地图工具的社区组织名称 |

## 系统域

| 中文 | English | 代码标识符 | 说明 |
| --- | --- | --- | --- |
| 系统用户 | system user | `sys_user` 表 | 注册用户 |
| 用户设备 | user device | `sys_user_device` 表 | 登录设备追踪（异常检测） |
| 用户邀请 | user invitation | `sys_user_invitation` 表 | 邀请码注册机制 |
| 操作日志 | action log | `sys_action_log` 表 | 用户操作审计 |
| 用户存档 | user archive | `sys_user_archive` 表 | 客户端存档槽位 |

## 标记与过滤

| 中文 | English | 代码标识符 | 说明 |
| --- | --- | --- | --- |
| 权限屏蔽标记 | hidden flag | `HiddenFlag` 枚举 | 数据级可见性：Visible(0)/Hidden(1)/Spy(2)/Suprise(3) |
| 特殊标记 | special flag | `special_flag: i32` | 位掩码过滤（物品/地区查询 UI） |
| 逻辑删除 | soft delete | `del_flag: bool` | 软删除标记，`find_safety()` 自动过滤 |
| 乐观锁 | optimistic lock | `version: i64` | 乐观锁版本号，`update_safety()` 自动校验+递增 |

## 审核流程

| 中文 | English | 说明 |
| --- | --- | --- |
| 暂存 | Pending (STAGE) | 打点提交状态之一：草稿，尚未提交审核 |
| 审核中 | Reviewing (COMMIT) | 打点提交状态之一：已提交，等待编辑审核 |
| 不通过 | Rejected (REJECT) | 打点提交状态之一：被驳回，附审核备注 |
| 新增 | Added | method_type：提交新点位 |
| 修改 | Modified | method_type：修改已有点位 |
| 删除 | Deleted | method_type：删除已有点位 |

## 技术概念

| 中文 | English | 说明 |
| --- | --- | --- |
| BinaryMD5 归档 | BinaryMD5 archive | GZIP 压缩的 JSON blob，以 MD5 为键，供客户端增量同步 |
| BinaryMD5 端点 | `*_doc` endpoints | `/api/item_doc`、`/api/marker_doc`、`/api/marker_link_doc` 等 |
| 冷启动 | cold start | 客户端首次加载，需下载全部点位/物品数据 |
| 增量同步 | incremental sync | 客户端比对 MD5 列表，仅获取变更的数据页 |
