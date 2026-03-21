use super::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::model::{MainMenuSummary, ReviewSummary};
use crate::reports::{ReportsPreview, has_visible_runtime_messages};
use crate::screen::{CommandMenu, ScreenId};
use ec_data::{CoreGameData, QueuedPlayerMail};

impl App {
    pub fn open_delete_reviewables(&mut self) {
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            !self.results_bytes.is_empty(),
            has_visible_runtime_messages(self.player.record_index_1_based as u8, &self.queued_mail),
        );
        let review_summary = ReviewSummary::from_main_menu(&summary);
        if !review_summary.reviewable_results && !review_summary.reviewable_messages {
            self.show_command_menu_notice(
                CommandMenu::General,
                "No messages or results are currently reviewable.",
            );
            return;
        }
        self.messaging.delete_reviewables_status = None;
        self.current_screen = ScreenId::DeleteReviewables;
    }

    pub fn open_compose_message_recipient(&mut self) {
        self.messaging.compose_recipient_input.clear();
        self.messaging.compose_recipient_status = None;
        self.messaging.compose_recipient_scroll_offset = 0;
        self.messaging.compose_recipient_cursor = 0;
        self.messaging.compose_recipient_empire = None;
        self.messaging.compose_subject.clear();
        self.messaging.compose_subject_status = None;
        self.messaging.compose_body.clear();
        self.messaging.compose_body_cursor = 0;
        self.messaging.compose_body_status = None;
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;
        self.messaging.compose_outbox_scroll_offset = 0;
        self.messaging.compose_sent_status = None;
        self.current_screen = ScreenId::ComposeMessageRecipient;
    }

    pub fn open_compose_message_subject(&mut self) {
        if self.messaging.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.messaging.compose_subject_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn open_compose_message_body(&mut self) {
        if self.messaging.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn open_compose_message_outbox(&mut self) {
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;
        self.messaging.compose_outbox_scroll_offset = 0;
        self.messaging.compose_outbox_cursor = 0;
        self.current_screen = ScreenId::ComposeMessageOutbox;
    }

    pub fn open_compose_message_discard_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.current_screen = ScreenId::ComposeMessageDiscardConfirm;
        }
    }

    pub fn open_compose_message_send_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            let body = self.messaging.compose_body.trim();
            if body.is_empty() {
                self.messaging.compose_body_status =
                    Some("Message body cannot be empty.".to_string());
                return;
            }
            self.current_screen = ScreenId::ComposeMessageSendConfirm;
        }
    }

    pub fn append_compose_recipient_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageRecipient
            && self.messaging.compose_recipient_input.len() < 2
        {
            self.messaging.compose_recipient_input.push(ch);
            self.messaging.compose_recipient_status = None;
        }
    }

    pub fn scroll_compose_recipients(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        let max_offset = total.saturating_sub(crate::screen::RECIPIENT_VISIBLE_ROWS);
        self.messaging.compose_recipient_scroll_offset = self
            .messaging
            .compose_recipient_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_recipient_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        if total == 0 {
            return;
        }
        let next = self.messaging.compose_recipient_cursor as isize + delta as isize;
        self.messaging.compose_recipient_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.messaging.compose_recipient_scroll_offset,
            self.messaging.compose_recipient_cursor,
            crate::screen::RECIPIENT_VISIBLE_ROWS,
        );
    }

    pub fn backspace_compose_recipient(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageRecipient {
            self.messaging.compose_recipient_input.pop();
            self.messaging.compose_recipient_status = None;
        }
    }

    pub fn submit_compose_recipient(&mut self) {
        // If the input box is empty, derive the empire id from the cursor row.
        let empire_id = if self.messaging.compose_recipient_input.trim().is_empty() {
            let ids: Vec<u8> = self
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
                .map(|(idx, _)| (idx + 1) as u8)
                .collect();
            match ids.get(self.messaging.compose_recipient_cursor) {
                Some(&id) => id,
                None => {
                    self.messaging.compose_recipient_status =
                        Some("No empire selected.".to_string());
                    return;
                }
            }
        } else {
            match self.messaging.compose_recipient_input.parse::<u8>() {
                Ok(id) => id,
                Err(_) => {
                    self.messaging.compose_recipient_status =
                        Some("Enter an empire number.".to_string());
                    return;
                }
            }
        };
        let max_empire = self.game_data.conquest.player_count();
        if !(1..=max_empire).contains(&empire_id) {
            self.messaging.compose_recipient_status =
                Some(format!("Enter an empire number in 1..={max_empire}."));
            return;
        }
        if empire_id as usize == self.player.record_index_1_based {
            self.messaging.compose_recipient_status =
                Some("You cannot message your own empire.".to_string());
            return;
        }
        self.messaging.compose_recipient_empire = Some(empire_id);
        self.messaging.compose_subject.clear();
        self.messaging.compose_subject_status = None;
        self.messaging.compose_body.clear();
        self.messaging.compose_body_cursor = 0;
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn append_compose_subject_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageSubject
            && self.messaging.compose_subject.chars().count() < crate::screen::COMPOSE_SUBJECT_LIMIT
        {
            self.messaging.compose_subject.push(ch);
            self.messaging.compose_subject_status = None;
        }
    }

    pub fn backspace_compose_subject(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageSubject {
            self.messaging.compose_subject.pop();
            self.messaging.compose_subject_status = None;
        }
    }

    pub fn submit_compose_subject(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageSubject {
            return;
        }
        self.messaging.compose_body_cursor = self.messaging.compose_body.chars().count();
        self.messaging.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn confirm_discard_composed_message(&mut self) {
        self.open_compose_message_recipient();
    }

    pub fn append_compose_body_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageBody
            && self.messaging.compose_body.chars().count() < crate::screen::COMPOSE_BODY_LIMIT
        {
            insert_char_at(
                &mut self.messaging.compose_body,
                self.messaging.compose_body_cursor,
                ch,
            );
            self.messaging.compose_body_cursor += 1;
            self.messaging.compose_body_status = None;
        } else if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
        }
    }

    pub fn backspace_compose_body(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            if self.messaging.compose_body_cursor > 0 {
                remove_char_before(
                    &mut self.messaging.compose_body,
                    self.messaging.compose_body_cursor,
                );
                self.messaging.compose_body_cursor -= 1;
            }
            self.messaging.compose_body_status = None;
        }
    }

    pub fn delete_compose_body_char(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            remove_char_at(
                &mut self.messaging.compose_body,
                self.messaging.compose_body_cursor,
            );
            self.messaging.compose_body_status = None;
        }
    }

    pub fn insert_compose_newline(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody
            && self.messaging.compose_body.chars().count() < crate::screen::COMPOSE_BODY_LIMIT
        {
            insert_char_at(
                &mut self.messaging.compose_body,
                self.messaging.compose_body_cursor,
                '\n',
            );
            self.messaging.compose_body_cursor += 1;
            self.messaging.compose_body_status = None;
        } else if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
        }
    }

    pub fn move_compose_body_cursor_left(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor =
                self.messaging.compose_body_cursor.saturating_sub(1);
        }
    }

    pub fn move_compose_body_cursor_right(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor = (self.messaging.compose_body_cursor + 1)
                .min(self.messaging.compose_body.chars().count());
        }
    }

    pub fn move_compose_body_cursor_home(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor = line_start_index(
                &self.messaging.compose_body,
                self.messaging.compose_body_cursor,
            );
        }
    }

    pub fn move_compose_body_cursor_end(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor = line_end_index(
                &self.messaging.compose_body,
                self.messaging.compose_body_cursor,
            );
        }
    }

    pub fn move_compose_body_cursor_up(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor = vertical_cursor_target(
                &self.messaging.compose_body,
                self.messaging.compose_body_cursor,
                -1,
            );
        }
    }

    pub fn move_compose_body_cursor_down(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.messaging.compose_body_cursor = vertical_cursor_target(
                &self.messaging.compose_body,
                self.messaging.compose_body_cursor,
                1,
            );
        }
    }

    pub fn send_composed_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageSendConfirm {
            return Ok(());
        }
        let Some(recipient_empire_id) = self.messaging.compose_recipient_empire else {
            self.messaging.compose_body_status = Some("Choose a recipient first.".to_string());
            return Ok(());
        };
        let body = self.messaging.compose_body.trim();
        if body.is_empty() {
            self.messaging.compose_body_status = Some("Message body cannot be empty.".to_string());
            return Ok(());
        }
        self.queued_mail.push(QueuedPlayerMail {
            sender_empire_id: self.player.record_index_1_based as u8,
            recipient_empire_id,
            year: self.game_data.conquest.game_year(),
            subject: self.messaging.compose_subject.trim().to_string(),
            body: body.to_string(),
            recipient_deleted: false,
        });
        self.save_game_data()?;
        self.messaging.compose_sent_status = Some(format!(
            "Message queued for Empire {recipient_empire_id}. It will be delivered after turn maintenance."
        ));
        self.current_screen = ScreenId::ComposeMessageSent;
        Ok(())
    }

    pub fn scroll_compose_outbox(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        let max_offset = total.saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
        self.messaging.compose_outbox_scroll_offset = self
            .messaging
            .compose_outbox_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_outbox_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        if total == 0 {
            return;
        }
        let next = self.messaging.compose_outbox_cursor as isize + delta as isize;
        self.messaging.compose_outbox_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.messaging.compose_outbox_scroll_offset,
            self.messaging.compose_outbox_cursor,
            crate::screen::OUTBOX_VISIBLE_ROWS,
        );
    }

    pub fn append_compose_outbox_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageOutbox
            && self.messaging.compose_outbox_input.len() < 2
        {
            self.messaging.compose_outbox_input.push(ch);
            self.messaging.compose_outbox_status = None;
        }
    }

    pub fn backspace_compose_outbox_input(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageOutbox {
            self.messaging.compose_outbox_input.pop();
            self.messaging.compose_outbox_status = None;
        }
    }

    pub fn delete_queued_compose_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return Ok(());
        }
        // If the input box is empty, use the cursor row (1-based queue_no).
        let queue_no = if self.messaging.compose_outbox_input.trim().is_empty() {
            self.messaging.compose_outbox_cursor + 1
        } else {
            let Ok(n) = self.messaging.compose_outbox_input.parse::<usize>() else {
                self.messaging.compose_outbox_status =
                    Some("Enter a queued message number.".to_string());
                return Ok(());
            };
            if n == 0 {
                self.messaging.compose_outbox_status =
                    Some("Enter a queued message number.".to_string());
                return Ok(());
            }
            n
        };

        let sender_empire_id = self.player.record_index_1_based as u8;
        let mut queue = self.queued_mail.clone();
        let own_indexes = queue
            .iter()
            .enumerate()
            .filter_map(|(idx, mail)| (mail.sender_empire_id == sender_empire_id).then_some(idx))
            .collect::<Vec<_>>();
        let Some(queue_index) = own_indexes.get(queue_no - 1).copied() else {
            self.messaging.compose_outbox_status = Some(format!(
                "Enter a queued message number in 1..={}.",
                own_indexes.len()
            ));
            return Ok(());
        };

        queue.remove(queue_index);
        self.queued_mail = queue;
        self.save_game_data()?;
        self.messaging.compose_outbox_input.clear();
        self.messaging.compose_outbox_status = None;

        // Clamp cursor and scroll offset to the new (smaller) queue.
        let new_len = own_indexes.len().saturating_sub(1);
        self.messaging.compose_outbox_cursor = self
            .messaging
            .compose_outbox_cursor
            .min(new_len.saturating_sub(1));
        let max_offset = new_len.saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
        self.messaging.compose_outbox_scroll_offset =
            self.messaging.compose_outbox_scroll_offset.min(max_offset);
        Ok(())
    }

    pub fn delete_reviewables(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.results_bytes.clear();
        for mail in &mut self.queued_mail {
            if mail.is_visible_to_recipient(self.player.record_index_1_based as u8) {
                mail.mark_deleted_by_recipient();
            }
        }
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.set_classic_login_reviewables_present(false);
            player.set_classic_results_chain_state(false, 0);
        }
        self.save_game_data()?;
        let refreshed = ReportsPreview::from_runtime(
            &self.game_data,
            self.player.record_index_1_based as u8,
            &self.results_bytes,
            &self.queued_mail,
        );
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            !self.results_bytes.is_empty(),
            has_visible_runtime_messages(self.player.record_index_1_based as u8, &self.queued_mail),
        );
        self.reports
            .replace(refreshed, ReviewSummary::from_main_menu(&summary));
        self.messaging.delete_reviewables_status =
            Some("Messages and results deleted.".to_string());
        Ok(())
    }

    pub(crate) fn compose_outbox_queue(
        &self,
    ) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
        let sender_empire_id = self.player.record_index_1_based as u8;
        Ok(self
            .queued_mail
            .clone()
            .into_iter()
            .filter(|mail| mail.sender_empire_id == sender_empire_id)
            .collect())
    }

    fn compose_outbox_queue_len(&self) -> usize {
        self.compose_outbox_queue()
            .map(|queue| queue.len())
            .unwrap_or(0)
    }
}

pub(crate) fn compose_recipient_label(game_data: &CoreGameData, empire_id: Option<u8>) -> String {
    let Some(empire_id) = empire_id else {
        return "<unknown>".to_string();
    };
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("Empire {empire_id} ({display})")
}

fn char_to_byte_index(body: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    body.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(body.len())
}

fn insert_char_at(body: &mut String, cursor_index: usize, ch: char) {
    let byte_index = char_to_byte_index(body, cursor_index);
    body.insert(byte_index, ch);
}

fn remove_char_before(body: &mut String, cursor_index: usize) {
    if cursor_index == 0 {
        return;
    }
    let start = char_to_byte_index(body, cursor_index - 1);
    let end = char_to_byte_index(body, cursor_index);
    body.replace_range(start..end, "");
}

fn remove_char_at(body: &mut String, cursor_index: usize) {
    let char_count = body.chars().count();
    if cursor_index >= char_count {
        return;
    }
    let start = char_to_byte_index(body, cursor_index);
    let end = char_to_byte_index(body, cursor_index + 1);
    body.replace_range(start..end, "");
}

fn line_start_index(body: &str, cursor_index: usize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let mut start = cursor_index.min(chars.len());
    while start > 0 && chars[start - 1] != '\n' {
        start -= 1;
    }
    start
}

fn line_end_index(body: &str, cursor_index: usize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let mut end = cursor_index.min(chars.len());
    while end < chars.len() && chars[end] != '\n' {
        end += 1;
    }
    end
}

fn vertical_cursor_target(body: &str, cursor_index: usize, delta: isize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let cursor = cursor_index.min(chars.len());
    let line_start = line_start_index(body, cursor);
    let line_end = line_end_index(body, cursor);
    let column = cursor.saturating_sub(line_start);

    let target_line_start = if delta < 0 {
        if line_start == 0 {
            return cursor;
        }
        let prev_end = line_start - 1;
        let mut prev_start = prev_end;
        while prev_start > 0 && chars[prev_start - 1] != '\n' {
            prev_start -= 1;
        }
        prev_start
    } else {
        if line_end == chars.len() {
            return cursor;
        }
        line_end + 1
    };

    let mut target_line_end = target_line_start;
    while target_line_end < chars.len() && chars[target_line_end] != '\n' {
        target_line_end += 1;
    }
    let target_len = target_line_end.saturating_sub(target_line_start);
    target_line_start + column.min(target_len)
}
