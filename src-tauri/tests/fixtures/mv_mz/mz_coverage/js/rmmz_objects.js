// Synthetic excerpt of the MZ core: the language setting and one reader.
Game_System.prototype.isCJK = function() {
    return $dataSystem.locale.match(/^(ja|zh|ko)/);
};

Game_Troop.prototype.letterTable = function() {
    return $gameSystem.isCJK() ? Game_Troop.LETTER_TABLE_FULL : Game_Troop.LETTER_TABLE_HALF;
};
