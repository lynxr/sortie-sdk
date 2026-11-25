//! The web API page: documents a logic publishes for a site and actions a site calls (API
//! revision 3).

use sortie::prelude::*;

use super::my_mission::{MyMission, airbases, groups, templates, zones};

// snippet:start web-api-publish
/// Что читает карта сайта: `GET /api/v1/docs/<логика>/front`.
#[derive(Serialize)]
pub struct Front {
    pub armor_left: usize,
    /// Широта и долгота центра бронегруппы, градусы.
    pub armor_at: Option<[f64; 2]>,
}

#[sortie::mission(api_rev = 3)] // ctx.publish — API ревизии 3
pub async fn run(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    loop {
        let (armor_left, center) = {
            let world = ctx.world();
            let armor = world.group(&groups::ARMOR);
            (armor.alive_count(), armor.center())
        };
        let armor_at = match center {
            Some(c) => {
                let (lat, lon, _alt) = ctx.dcs().coord().lo_to_ll(c.alt(m(0.0))).await?;
                Some([lat, lon])
            }
            None => None,
        };
        // Те же данные ещё раз ничего не меняют: rev прежний, поток документа молчит.
        let front = Front {
            armor_left,
            armor_at,
        };
        ctx.publish("front", &front).await?;
        ctx.sleep(secs(10)).await;
    }
}
// snippet:end web-api-publish

pub async fn unpublish(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start web-api-unpublish
    let removed = ctx.unpublish("front").await?; // false — такого документа не было
    // snippet:end web-api-unpublish
    let _ = removed;
    Ok(())
}

// snippet:start web-api-action
/// Что присылает сайт: `POST /api/v1/actions/<логика>/send_cap`.
#[derive(Deserialize)]
pub struct SendCap {
    /// Его номер заказа: повтор того же заказа (после 504) не поднимет второй патруль.
    pub order: String,
    /// Центр патруля в метрах DCS: `x` — север, `z` — восток.
    pub x: f64,
    pub z: f64,
}

/// Что получает сайт в ответе 200.
#[derive(Serialize)]
pub struct CapSent {
    pub group: String,
}

#[sortie::mission(api_rev = 3)] // ctx.api() — API ревизии 3
pub async fn dispatcher(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let mut send_cap = ctx.api().action("send_cap")?;
    while let Some(req) = send_cap.next().await {
        let order: SendCap = match req.json() {
            Ok(order) => order,
            Err(e) => {
                req.fail(400, e)?; // тело не подходит: 400, code "refused"
                continue;
            }
        };
        let center = MapPoint::new(order.x, order.z);
        if !zones::PATROL.contains(center) {
            req.fail(422, "the point is outside the patrol zone")?;
            continue;
        }
        let cap = ctx
            .spawn_group(templates::RED_CAP.named(format!("Red CAP {}", order.order)))
            .from_airbase(airbases::NALCHIK, StartType::Runway)
            .key(format!("send-cap-{}", order.order))
            .await?;
        cap.push_task(Task::orbit(center)).await?;
        req.reply(&CapSent {
            group: cap.name().to_owned(),
        })?;
    }
    Ok(())
}
// snippet:end web-api-action
